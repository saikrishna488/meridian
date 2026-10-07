//! Installed-application catalog, built from XDG desktop entries via GIO.
//!
//! GIO implements the desktop-entry spec for us: `$XDG_DATA_DIRS` lookup
//! (including Flatpak exports), `NoDisplay`/`Hidden`/`OnlyShowIn` filtering,
//! localization, and, at launch time, `Exec` field-code expansion without a
//! shell.
//!
//! Development in a toolbox container: the host's applications are visible
//! under `/run/host`, but their programs are not installed in the container,
//! so GIO (which hides entries whose `Exec` program is missing) can't list
//! them. With `MERIDIAN_HOST_APPS=1` (set by `tools/dev-session.sh`), the
//! catalog reads the host's desktop entries itself (see [`load_host_apps`])
//! and launches them *on the host* with
//! `flatpak-spawn --host gio launch <desktop file>`, a fixed argument vector
//! whose only variable is a desktop-file path found by the catalog.

use std::path::{Path, PathBuf};

use gio::prelude::*;

use crate::search::apps::SearchFields;

#[derive(Debug, Clone)]
pub struct App {
    /// Desktop-file id, e.g. `org.mozilla.firefox.desktop`.
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub generic_name: Option<String>,
    pub keywords: Vec<String>,
    pub description: Option<String>,
    /// `StartupWMClass`, used to match windows to apps.
    pub wm_class: Option<String>,
    pub(crate) search: SearchFields,
    info: Option<gio_unix::DesktopAppInfo>,
    pub source: Option<PathBuf>,
}

impl App {
    fn from_info(info: gio_unix::DesktopAppInfo) -> Option<App> {
        let id = info.id()?.to_string();
        let name = info.display_name().to_string();
        let generic_name = info.generic_name().map(|s| s.to_string()).filter(|s| !s.is_empty());
        let keywords: Vec<String> = info.keywords().iter().map(|s| s.to_string()).collect();
        let description = info.description().map(|s| s.to_string()).filter(|s| !s.is_empty());
        let search = SearchFields::new(&name, generic_name.as_deref(), &keywords, description.as_deref());
        let wm_class = info.startup_wm_class().map(|s| s.to_string());
        let source = info.filename();
        Some(App {
            id,
            name,
            icon: info.icon().and_then(|i| i.to_string()).map(|s| s.to_string()),
            generic_name,
            keywords,
            description,
            wm_class,
            search,
            info: Some(info),
            source,
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(name: &str, generic: Option<&str>, keywords: &[String], description: Option<&str>) -> App {
        App {
            id: format!("{}.desktop", name.to_lowercase().replace(' ', "-")),
            name: name.into(),
            icon: None,
            generic_name: generic.map(Into::into),
            keywords: keywords.to_vec(),
            description: description.map(Into::into),
            wm_class: None,
            search: SearchFields::new(name, generic, keywords, description),
            info: None,
            source: None,
        }
    }
}

#[derive(Debug)]
pub enum LaunchError {
    /// The id is not in the catalog (never launched, even if a desktop file
    /// with that name exists elsewhere).
    NotFound,
    Failed(String),
}

/// Launches host applications from inside a toolbox container.
#[derive(Debug, Clone)]
pub struct HostBridge {
    /// Environment forwarded to host apps so they open in our session.
    env: Vec<(String, String)>,
}

impl HostBridge {
    /// Enabled by `MERIDIAN_HOST_APPS=1`.
    pub fn from_env() -> Option<HostBridge> {
        if std::env::var("MERIDIAN_HOST_APPS").ok().as_deref() != Some("1") {
            return None;
        }
        let env = ["WAYLAND_DISPLAY", "DISPLAY", "XDG_CURRENT_DESKTOP"]
            .into_iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_owned(), v)))
            .collect();
        Some(HostBridge { env })
    }

    /// The host-side path of a desktop file, if it belongs to the host.
    fn host_path(source: &Path) -> Option<PathBuf> {
        if let Ok(rest) = source.strip_prefix("/run/host") {
            return Some(Path::new("/").join(rest));
        }
        // The home directory is shared with the host.
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        source.starts_with(&home).then(|| source.to_path_buf())
    }

    fn launch(&self, host_path: &Path) -> Result<(), LaunchError> {
        let mut argv: Vec<std::ffi::OsString> = vec!["flatpak-spawn".into(), "--host".into()];
        argv.extend(self.env.iter().map(|(k, v)| format!("--env={k}={v}").into()));
        argv.extend(["gio".into(), "launch".into(), host_path.as_os_str().to_owned()]);
        let argv: Vec<&std::ffi::OsStr> = argv.iter().map(|a| a.as_os_str()).collect();
        let proc = gio::Subprocess::newv(&argv, gio::SubprocessFlags::NONE)
            .map_err(|e| LaunchError::Failed(format!("flatpak-spawn: {e}")))?;
        let path = host_path.display().to_string();
        proc.wait_check_async(gio::Cancellable::NONE, move |r| {
            if let Err(e) = r {
                log::warn!("host launch of {path} failed: {e}");
            }
        });
        Ok(())
    }
}

#[derive(Default)]
pub struct AppCatalog {
    apps: Vec<App>,
    host: Option<HostBridge>,
}

impl AppCatalog {
    /// Load all applications that should be shown in this desktop.
    pub fn load(host: Option<HostBridge>) -> Self {
        let local = gio::AppInfo::all()
            .into_iter()
            .filter(|a| a.should_show())
            .filter_map(|a| a.downcast::<gio_unix::DesktopAppInfo>().ok())
            .filter_map(App::from_info);
        let mut apps: Vec<App> = match &host {
            // Host entries take precedence over the container's.
            Some(_) => {
                let mut apps = load_host_apps();
                let seen: std::collections::HashSet<String> = apps.iter().map(|a| a.id.clone()).collect();
                apps.extend(local.filter(|a| !seen.contains(&a.id)));
                apps
            }
            None => local.collect(),
        };
        apps.sort_by_cached_key(|a| a.name.to_lowercase());
        log::info!("app catalog: {} applications{}", apps.len(), if host.is_some() { " (host bridge)" } else { "" });
        AppCatalog { apps, host }
    }

    /// All applications, sorted by display name.
    pub fn apps(&self) -> &[App] {
        &self.apps
    }

    pub fn get(&self, id: &str) -> Option<&App> {
        self.apps.iter().find(|a| a.id == id)
    }

    /// Launch an application by desktop-file id.
    pub fn launch(&self, id: &str, ctx: Option<&gio::AppLaunchContext>) -> Result<(), LaunchError> {
        let app = self.get(id).ok_or(LaunchError::NotFound)?;
        if let (Some(host), Some(path)) = (&self.host, app.source.as_deref().and_then(HostBridge::host_path)) {
            log::info!("launching {id} on the host ({})", path.display());
            return host.launch(&path);
        }
        let info = app.info.as_ref().ok_or(LaunchError::NotFound)?;
        log::info!("launching {id}");
        info.launch(&[], ctx).map_err(|e| LaunchError::Failed(e.to_string()))
    }

    /// Display name for a window's Wayland app id (or X11 WM class).
    pub fn name_for_window(&self, app_id: &str) -> Option<&str> {
        if app_id.is_empty() {
            return None;
        }
        let desktop_id = format!("{app_id}.desktop");
        self.apps
            .iter()
            .find(|a| a.id.eq_ignore_ascii_case(&desktop_id))
            .or_else(|| {
                self.apps.iter().find(|a| a.wm_class.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(app_id)))
            })
            .or_else(|| {
                // Reverse-DNS ids: "org.gnome.TextEditor" → "texteditor"-ish match.
                let last = app_id.rsplit('.').next()?;
                self.apps.iter().find(|a| a.id.trim_end_matches(".desktop").rsplit('.').next() == Some(last))
            })
            .map(|a| a.name.as_str())
    }
}

// ---- host desktop entries (toolbox development) ----------------------------

/// The host root as seen from the container.
const HOST_ROOT: &str = "/run/host";

/// Host data directories in desktop-entry precedence order: the user's own
/// (the home directory is shared with the container), then the system's,
/// following Fedora's default `XDG_DATA_DIRS`.
fn host_data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    vec![
        home.join(".local/share"),
        home.join(".local/share/flatpak/exports/share"),
        PathBuf::from(HOST_ROOT).join("var/lib/flatpak/exports/share"),
        PathBuf::from(HOST_ROOT).join("usr/local/share"),
        PathBuf::from(HOST_ROOT).join("usr/share"),
    ]
}

/// Read the host's desktop entries, applying the desktop-entry spec rules
/// GIO would apply on the host: id precedence (a higher-precedence entry,
/// even a `Hidden` one, masks lower ones), `Type`, `Hidden`, `NoDisplay`,
/// `OnlyShowIn`/`NotShowIn`, and `TryExec` (checked on the host's filesystem).
fn load_host_apps() -> Vec<App> {
    let desktops: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut apps = Vec::new();
    for dir in host_data_dirs() {
        let base = dir.join("applications");
        let mut files = Vec::new();
        collect_desktop_files(&base, &mut files);
        files.sort();
        for path in files {
            let Some(id) = desktop_id(&base, &path) else { continue };
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(app) = App::from_host_file(id, &path, &desktops) {
                apps.push(app);
            }
        }
    }
    apps
}

fn collect_desktop_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_desktop_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "desktop") {
            out.push(path);
        }
    }
}

/// Desktop-file id: path relative to `applications/`, with `/` → `-`.
fn desktop_id(base: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(base).ok()?.to_str()?;
    Some(rel.replace('/', "-"))
}

impl App {
    fn from_host_file(id: String, path: &Path, desktops: &[String]) -> Option<App> {
        const GROUP: &str = "Desktop Entry";
        let kf = glib::KeyFile::new();
        kf.load_from_file(path, glib::KeyFileFlags::NONE).ok()?;
        let get_bool = |key| kf.boolean(GROUP, key).unwrap_or(false);
        let get_str = |key| kf.locale_string(GROUP, key, None).ok().map(|s| s.to_string()).filter(|s| !s.is_empty());
        let get_list = |key| kf.string_list(GROUP, key).map(|l| l.iter().map(|s| s.to_string()).collect::<Vec<_>>());

        if kf.string(GROUP, "Type").ok()?.as_str() != "Application" || get_bool("Hidden") || get_bool("NoDisplay") {
            return None;
        }
        if let Ok(only) = get_list("OnlyShowIn")
            && !only.iter().any(|d| desktops.contains(d))
        {
            return None;
        }
        if let Ok(not) = get_list("NotShowIn")
            && not.iter().any(|d| desktops.contains(d))
        {
            return None;
        }
        if let Some(try_exec) = kf.string(GROUP, "TryExec").ok().filter(|s| !s.is_empty())
            && !host_program_exists(&try_exec)
        {
            return None;
        }

        let name = get_str("Name")?;
        let generic_name = get_str("GenericName");
        let keywords: Vec<String> = kf
            .locale_string_list(GROUP, "Keywords", None)
            .map(|l| l.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let description = get_str("Comment");
        let wm_class = kf.string(GROUP, "StartupWMClass").ok().map(|s| s.to_string());
        let search = SearchFields::new(&name, generic_name.as_deref(), &keywords, description.as_deref());
        Some(App {
            id,
            name,
            icon: get_str("Icon"),
            generic_name,
            keywords,
            description,
            wm_class,
            search,
            info: None,
            source: Some(path.to_path_buf()),
        })
    }
}

/// Whether a `TryExec` program exists on the host.
fn host_program_exists(program: &str) -> bool {
    let host = Path::new(HOST_ROOT);
    if program.starts_with('/') {
        return host.join(program.trim_start_matches('/')).exists();
    }
    ["usr/local/bin", "usr/bin", "bin", "usr/local/sbin", "usr/sbin"]
        .iter()
        .any(|d| host.join(d).join(program).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_paths() {
        assert_eq!(
            HostBridge::host_path(Path::new("/run/host/usr/share/applications/firefox.desktop")),
            Some(PathBuf::from("/usr/share/applications/firefox.desktop"))
        );
        // Container-local entries are launched locally.
        assert_eq!(HostBridge::host_path(Path::new("/usr/share/applications/foot.desktop")), None);
    }

    #[test]
    fn window_names() {
        let mut firefox = App::for_test("Firefox", None, &[], None);
        firefox.id = "org.mozilla.firefox.desktop".into();
        let mut editor = App::for_test("Text Editor", None, &[], None);
        editor.wm_class = Some("gedit".into());
        let catalog = AppCatalog { apps: vec![firefox, editor], host: None };
        assert_eq!(catalog.name_for_window("org.mozilla.firefox"), Some("Firefox"));
        assert_eq!(catalog.name_for_window("Gedit"), Some("Text Editor"));
        assert_eq!(catalog.name_for_window("firefox"), Some("Firefox"));
        assert_eq!(catalog.name_for_window("unknown.app"), None);
        assert_eq!(catalog.name_for_window(""), None);
    }

    #[test]
    fn parses_host_desktop_entries() {
        let dir = std::env::temp_dir().join(format!("meridian-apps-test-{}", std::process::id()));
        let apps = dir.join("applications");
        std::fs::create_dir_all(apps.join("sub")).unwrap();
        let write = |name: &str, body: &str| std::fs::write(apps.join(name), body).unwrap();
        write(
            "editor.desktop",
            "[Desktop Entry]\nType=Application\nName=Editor\nGenericName=Text Editor\nKeywords=txt;notes;\nExec=editor %F\n",
        );
        write("hidden.desktop", "[Desktop Entry]\nType=Application\nName=Hidden\nNoDisplay=true\nExec=x\n");
        write("kde-only.desktop", "[Desktop Entry]\nType=Application\nName=KDE\nOnlyShowIn=KDE;\nExec=x\n");
        write("link.desktop", "[Desktop Entry]\nType=Link\nName=Link\nURL=https://example.org\n");
        write("sub/nested.desktop", "[Desktop Entry]\nType=Application\nName=Nested\nExec=x\n");
        let desktops = vec!["Meridian".to_owned()];
        let load = |name: &str| App::from_host_file(name.into(), &apps.join(name), &desktops);

        let editor = load("editor.desktop").unwrap();
        assert_eq!(editor.name, "Editor");
        assert_eq!(editor.generic_name.as_deref(), Some("Text Editor"));
        assert_eq!(editor.keywords, vec!["txt", "notes"]);
        assert!(load("hidden.desktop").is_none());
        assert!(load("kde-only.desktop").is_none());
        assert!(load("link.desktop").is_none());
        assert_eq!(desktop_id(&apps, &apps.join("sub/nested.desktop")).as_deref(), Some("sub-nested.desktop"));
    }
}
