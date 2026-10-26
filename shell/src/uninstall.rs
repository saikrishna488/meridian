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

pub(crate) async fn run(args: &[&str]) -> Result<String, ErrorBody> {
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
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || b"._+:-".contains(&c))
}

fn valid_snap(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value.len() <= snap_name_limit()
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

const fn snap_name_limit() -> usize {
    40
}

fn unique_debian_owner(output: &str) -> Option<String> {
    let mut owners = std::collections::BTreeSet::new();
    for line in output.lines() {
        let Some((package, _)) = line.split_once(": ") else { continue };
        if valid_package(package) {
            owners.insert(package.to_owned());
        }
    }
    (owners.len() == 1).then(|| owners.into_iter().next().unwrap())
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
    } else if let Ok(name) = desktop
        .string("Desktop Entry", "X-SnapInstanceName")
        .or_else(|_| desktop.string("Desktop Entry", "X-SnapAppName"))
    {
        if !valid_snap(&name) {
            return Err(error("Invalid Snap application name"));
        }
        run(&["snap", "info", &name]).await?;
        (
            format!("snap:{name}"),
            format!("Removes the Snap package {name} and its application data. Authentication may be required."),
        )
    } else {
        let path = source
            .strip_prefix("/run/host")
            .map(|p| std::path::Path::new("/").join(p))
            .unwrap_or_else(|_| source.clone());
        let path = path.to_str().ok_or_else(|| error("Invalid desktop file path"))?;
        // Local launchers (including AppImage shortcuts) are not owned by dpkg.
        // Keep dpkg-query's diagnostic out of the confirmation UI: it commonly
        // includes a confusing "no path found" message for these entries.
        let owner = run(&["dpkg-query", "--search", "--", path]).await.map_err(|_| {
            error("This application is not installed by a supported package manager, so Meridian cannot uninstall it.")
        })?;
        let package = unique_debian_owner(&owner).ok_or_else(|| {
            error("This application is not installed by a supported package manager, so Meridian cannot uninstall it.")
        })?;
        (
            format!("deb:{package}"),
            format!("Purges the {package} package and removes unused dependencies. Authentication may be required."),
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
    } else if let Some(package) = expected.strip_prefix("deb:") {
        if !valid_package(package) {
            return Err(error("Invalid Debian package name"));
        }
        run(&["pkexec", "apt-get", "purge", "--yes", "--auto-remove", "--", package]).await?;
    } else if let Some(name) = expected.strip_prefix("snap:") {
        if !valid_snap(name) {
            return Err(error("Invalid Snap application name"));
        }
        run(&["pkexec", "snap", "remove", "--purge", name]).await?;
    } else {
        return Err(error("Unsupported uninstall target"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_names_cannot_supply_options_or_shell_code() {
        assert!(valid_package("org.mozilla.firefox"));
        assert!(valid_package("firefox-144.0+1"));
        assert!(valid_package("firefox:amd64"));
        for value in ["", "--all", "a;b", "$(id)", "a b", "a\nb", "a/b"] {
            assert!(!valid_package(value));
        }
    }

    #[test]
    fn debian_owner_must_be_unique() {
        assert_eq!(
            unique_debian_owner("firefox:amd64: /usr/share/applications/firefox.desktop\n"),
            Some("firefox:amd64".into())
        );
        assert_eq!(
            unique_debian_owner(
                "firefox: /usr/share/applications/firefox.desktop\nfirefox: /usr/share/icons/firefox.svg\n"
            ),
            Some("firefox".into())
        );
        assert_eq!(
            unique_debian_owner("one: /usr/share/applications/a.desktop\ntwo: /usr/share/applications/a.desktop\n"),
            None
        );
    }

    #[test]
    fn snap_names_are_constrained_to_package_names() {
        assert!(valid_snap("firefox"));
        assert!(valid_snap("my-app-2"));
        assert!(!valid_snap(""));
        assert!(!valid_snap("my_app"));
        assert!(!valid_snap("--bad"));
    }
}
