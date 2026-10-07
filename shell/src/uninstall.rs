//! Package-manager uninstallation. Targets come from catalog desktop files,
//! never from UI command strings. No desktop files are deleted as a shortcut.
use gtk::gio;
use meridian_protocol::{ErrorBody, ErrorCode, UninstallPlan};
use meridian_services::apps::App;

fn error(message: impl Into<String>) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, message)
}

fn host_command(args: &[&str]) -> Vec<std::ffi::OsString> {
    let mut argv = Vec::new();
    if std::env::var("MERIDIAN_HOST_APPS").as_deref() == Ok("1") {
        argv.extend(["flatpak-spawn".into(), "--host".into()]);
    }
    argv.extend(args.iter().map(std::ffi::OsString::from));
    argv
}

async fn run(args: &[&str]) -> Result<String, ErrorBody> {
    let argv = host_command(args);
    let refs: Vec<_> = argv.iter().map(|arg| arg.as_os_str()).collect();
    let process = gio::Subprocess::newv(&refs, gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE)
        .map_err(|e| error(format!("Could not start package manager: {e}")))?;
    let (stdout, stderr) = process.communicate_utf8_future(None).await.map_err(|e| error(e.to_string()))?;
    if !process.is_successful() {
        return Err(error(
            stderr
                .or(stdout)
                .map(|text| text.chars().take(1800).collect::<String>())
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "Package manager cancelled or failed".into()),
        ));
    }
    Ok(stdout.map(|text| text.to_string()).unwrap_or_default())
}

fn valid_package(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || b"._+-".contains(&c))
}

pub async fn plan(app: &App) -> Result<UninstallPlan, ErrorBody> {
    let source = app.source.as_ref().ok_or_else(|| error("This app is not managed by a supported package manager"))?;
    let desktop = gio::glib::KeyFile::new();
    desktop.load_from_file(source, gio::glib::KeyFileFlags::NONE).map_err(|e| error(e.to_string()))?;
    let (target, detail) = if let Ok(id) = desktop.string("Desktop Entry", "X-Flatpak") {
        if !valid_package(&id) {
            return Err(error("Invalid Flatpak application ID"));
        }
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
        let scope = if source.starts_with(home.join(".local/share/flatpak"))
            || source.starts_with(gio::glib::user_data_dir().join("flatpak"))
        {
            "--user"
        } else {
            "--system"
        };
        // Resolve the exact installed app ref (including architecture and branch).
        let reference = run(&["flatpak", "info", scope, "--show-ref", &id]).await?.trim().to_owned();
        if !reference.starts_with(&format!("app/{id}/")) || reference.split('/').count() != 4 {
            return Err(error("Could not resolve the installed Flatpak application"));
        }
        (format!("flatpak:{scope}:{reference}"), "Removes the application. Your saved app data will be kept.".into())
    } else {
        let path = source
            .strip_prefix("/run/host")
            .map(|p| std::path::Path::new("/").join(p))
            .unwrap_or_else(|_| source.clone());
        let path = path.to_str().ok_or_else(|| error("Invalid desktop file path"))?;
        let package = run(&["rpm", "-qf", "--qf", "%{NAME}", path]).await?.trim().to_owned();
        if !valid_package(&package) {
            return Err(error("Could not identify a unique RPM package"));
        }
        (
            format!("rpm:{package}"),
            format!(
                "Removes the {package} package and any packages that depend on it. Authentication may be required."
            ),
        )
    };
    Ok(UninstallPlan { id: app.id.clone(), name: app.name.clone(), target, detail })
}

thread_local! { static UNINSTALLING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
struct UninstallGuard;
impl Drop for UninstallGuard {
    fn drop(&mut self) {
        UNINSTALLING.set(false);
    }
}

pub async fn uninstall(app: &App, expected: &str) -> Result<(), ErrorBody> {
    if UNINSTALLING.replace(true) {
        return Err(error("Another application is being uninstalled. Wait for it to finish."));
    }
    let _guard = UninstallGuard;
    let plan = plan(app).await?;
    if plan.target != expected {
        return Err(error("The installation changed. Review the uninstall confirmation again."));
    }
    if let Some(target) = expected.strip_prefix("flatpak:") {
        let (scope, reference) = target.split_once(':').ok_or_else(|| error("Invalid uninstall target"))?;
        run(&["flatpak", "uninstall", scope, "--noninteractive", "--app", reference]).await?;
    } else if let Some(package) = expected.strip_prefix("rpm:") {
        run(&["pkexec", "dnf", "remove", "--assumeyes", "--setopt=clean_requirements_on_remove=False", package])
            .await?;
    } else {
        return Err(error("Unsupported uninstall target"));
    }
    Ok(())
}

pub fn open_trash() -> Result<(), ErrorBody> {
    let argv = host_command(&["gio", "open", "trash:///"]);
    let refs: Vec<_> = argv.iter().map(|arg| arg.as_os_str()).collect();
    let process = gio::Subprocess::newv(&refs, gio::SubprocessFlags::NONE).map_err(|e| error(e.to_string()))?;
    process.wait_check_async(gio::Cancellable::NONE, |result| {
        if let Err(e) = result {
            log::warn!("could not open Trash: {e}");
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_names_cannot_supply_options_or_shell_code() {
        assert!(valid_package("org.mozilla.firefox"));
        assert!(valid_package("firefox-144.0+1"));
        for value in ["", "--all", "a;b", "$(id)", "a b", "a\nb", "a/b"] {
            assert!(!valid_package(value));
        }
    }
}
