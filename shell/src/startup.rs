//! XDG login applications. Changes are per-user overrides, never system edits.
use gio::prelude::*;
use gtk::{gio, glib};
use meridian_protocol::{ErrorBody, ErrorCode, StartupEntry};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

const GROUP: &str = "Desktop Entry";
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
fn valid_id(id: &str) -> bool {
    id.ends_with(".desktop") && id.len() <= 255 && !id.starts_with('.') && !id.contains(['/', '\\', '\0'])
}
fn roots() -> Vec<PathBuf> {
    std::iter::once(glib::user_config_dir().join("autostart"))
        .chain(glib::system_config_dirs().into_iter().map(|p| p.join("autostart")))
        .collect()
}
fn load(path: &Path) -> Result<glib::KeyFile, ErrorBody> {
    let file = glib::KeyFile::new();
    file.load_from_file(path, glib::KeyFileFlags::KEEP_COMMENTS | glib::KeyFileFlags::KEEP_TRANSLATIONS)
        .map_err(error)?;
    Ok(file)
}
fn text(file: &glib::KeyFile, key: &str) -> String {
    file.string(GROUP, key).map(|s| s.to_string()).unwrap_or_default()
}
fn boolean(file: &glib::KeyFile, key: &str) -> bool {
    file.boolean(GROUP, key).unwrap_or(false)
}
struct Entry {
    item: StartupEntry,
    path: PathBuf,
}
fn scan(dirs: &[PathBuf], desktops: &[&str]) -> Vec<Entry> {
    let mut entries = BTreeMap::new();
    let mut seen = HashSet::new();
    // First directory wins, including disabled user overrides.
    for (index, directory) in dirs.iter().enumerate() {
        let Ok(files) = std::fs::read_dir(directory) else { continue };
        for path in files.flatten().map(|e| e.path()) {
            let Some(id) = path.file_name().and_then(|s| s.to_str()).filter(|id| valid_id(id)).map(str::to_owned)
            else {
                continue;
            };
            if !seen.insert(id.clone()) {
                continue;
            }
            let Ok(file) = load(&path) else { continue };
            // Some desktop environments write a minimal Hidden=true override.
            let fallback = dirs.iter().skip(index + 1).find_map(|d| load(&d.join(&id)).ok());
            let metadata = if text(&file, "Name").is_empty() { fallback.as_ref().unwrap_or(&file) } else { &file };
            if text(metadata, "Type") != "Application" {
                continue;
            }
            let only = text(metadata, "OnlyShowIn");
            let not = text(metadata, "NotShowIn");
            let matches = |list: &str| list.split(';').any(|item| desktops.contains(&item));
            let try_exec = text(metadata, "TryExec");
            let command = text(metadata, "Exec");
            let reason = if !only.is_empty() && !matches(&only) {
                "For another desktop environment"
            } else if matches(&not) {
                "Excluded from this desktop environment"
            } else if !try_exec.is_empty() && glib::find_program_in_path(&try_exec).is_none() {
                "Required program is not installed"
            } else if command.is_empty() && !boolean(metadata, "DBusActivatable") {
                "No startup command"
            } else {
                ""
            };
            let name =
                metadata.locale_string(GROUP, "Name", None).map(|s| s.to_string()).unwrap_or_else(|_| id.clone());
            let description = metadata.locale_string(GROUP, "Comment", None).map(|s| s.to_string()).unwrap_or_default();
            entries.insert(
                id.clone(),
                Entry {
                    item: StartupEntry {
                        id,
                        name,
                        description,
                        command,
                        enabled: !boolean(&file, "Hidden")
                            && file.boolean(GROUP, "X-GNOME-Autostart-enabled").unwrap_or(true),
                        available: reason.is_empty(),
                        reason: reason.into(),
                        user: index == 0,
                    },
                    path,
                },
            );
        }
    }
    let mut entries: Vec<_> = entries.into_values().collect();
    entries.sort_by_key(|e| e.item.name.to_lowercase());
    entries
}
fn desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_else(|_| "Meridian:wlroots".into())
        .split(':')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
pub fn list() -> Result<Vec<StartupEntry>, ErrorBody> {
    let desktops = desktops();
    Ok(scan(&roots(), &desktops.iter().map(String::as_str).collect::<Vec<_>>()).into_iter().map(|e| e.item).collect())
}
fn write(file: &glib::KeyFile, target: &Path) -> Result<(), ErrorBody> {
    std::fs::create_dir_all(target.parent().ok_or_else(|| error("Invalid startup location"))?).map_err(error)?;
    gio::File::for_path(target)
        .replace_contents(
            file.to_data().as_bytes(),
            None,
            false,
            gio::FileCreateFlags::PRIVATE | gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::Cancellable::NONE,
        )
        .map_err(error)?;
    Ok(())
}
fn set_at(dirs: &[PathBuf], id: &str, enabled: bool, desktops: &[&str]) -> Result<(), ErrorBody> {
    if !valid_id(id) {
        return Err(error("Invalid startup application"));
    }
    let entry = scan(dirs, desktops)
        .into_iter()
        .find(|e| e.item.id == id)
        .ok_or_else(|| error("Startup application no longer exists"))?;
    if enabled && !entry.item.available {
        return Err(error(&entry.item.reason));
    }
    let mut file = load(&entry.path)?;
    if text(&file, "Name").is_empty() {
        file = dirs
            .iter()
            .skip(1)
            .find_map(|d| load(&d.join(id)).ok())
            .ok_or_else(|| error("Original startup application is missing"))?;
    }
    file.set_boolean(GROUP, "Hidden", !enabled);
    file.set_boolean(GROUP, "X-GNOME-Autostart-enabled", enabled);
    write(&file, &dirs[0].join(id))
}
pub fn set(id: &str, enabled: bool) -> Result<Vec<StartupEntry>, ErrorBody> {
    let names = desktops();
    set_at(&roots(), id, enabled, &names.iter().map(String::as_str).collect::<Vec<_>>())?;
    list()
}
pub fn add(id: &str, source: &Path) -> Result<Vec<StartupEntry>, ErrorBody> {
    if !valid_id(id) {
        return Err(error("Invalid application id"));
    }
    let target = roots()[0].join(id);
    if std::fs::symlink_metadata(&target).is_ok() {
        return Err(error("This application already has a startup entry. Use its switch to enable it."));
    }
    let file = load(source)?;
    if text(&file, "Type") != "Application" {
        return Err(error("Choose an installed application"));
    }
    file.set_boolean(GROUP, "Hidden", false);
    file.set_boolean(GROUP, "X-GNOME-Autostart-enabled", true);
    write(&file, &target)?;
    list()
}
/// Called once by labwc's session autostart, never by shell reactivation.
pub fn launch() {
    let desktops = desktops();
    for entry in scan(&roots(), &desktops.iter().map(String::as_str).collect::<Vec<_>>()) {
        if !entry.item.enabled || !entry.item.available {
            continue;
        }
        match gio_unix::DesktopAppInfo::from_filename(&entry.path) {
            Some(info) => {
                if let Err(e) = info.launch(&[], None::<&gio::AppLaunchContext>) {
                    log::warn!("Could not start {}: {e}", entry.item.id);
                }
            }
            None => log::warn!("Invalid startup entry: {}", entry.item.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_overrides_preserve_system_entries_and_desktop_filters() {
        let root = std::env::temp_dir().join(format!("meridian-startup-{}", std::process::id()));
        let dirs = vec![root.join("user"), root.join("system")];
        for d in &dirs {
            std::fs::create_dir_all(d).unwrap();
        }
        let source = "[Desktop Entry]\nType=Application\nName=Example\nExec=/bin/true\nComment=Start me\n";
        std::fs::write(dirs[1].join("example.desktop"), source).unwrap();
        std::fs::write(dirs[1].join("other.desktop"), format!("{source}OnlyShowIn=GNOME;\n")).unwrap();
        std::fs::write(dirs[1].join("missing.desktop"), format!("{source}TryExec=meridian-nonexistent-program-1234\n"))
            .unwrap();
        let entries = scan(&dirs, &["Meridian"]);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries.iter().filter(|e| e.item.available).count(), 1);
        assert!(set_at(&dirs, "other.desktop", true, &["Meridian"]).is_err());
        set_at(&dirs, "example.desktop", false, &["Meridian"]).unwrap();
        assert_eq!(std::fs::read_to_string(dirs[1].join("example.desktop")).unwrap(), source);
        assert!(!scan(&dirs, &["Meridian"]).iter().find(|e| e.item.id == "example.desktop").unwrap().item.enabled);
        set_at(&dirs, "example.desktop", true, &["Meridian"]).unwrap();
        assert!(scan(&dirs, &["Meridian"]).iter().find(|e| e.item.id == "example.desktop").unwrap().item.enabled);
        std::fs::write(dirs[0].join("example.desktop"), "[Desktop Entry]\nHidden=true\n").unwrap();
        set_at(&dirs, "example.desktop", true, &["Meridian"]).unwrap();
        assert_eq!(text(&load(&dirs[0].join("example.desktop")).unwrap(), "Exec"), "/bin/true");
        for id in ["../outside.desktop", "/tmp/app.desktop", "bad\\name.desktop", ".desktop", "file.txt"] {
            assert!(set_at(&dirs, id, true, &["Meridian"]).is_err());
        }
        std::fs::write(dirs[0].join("example.desktop"), "not a desktop file").unwrap();
        assert!(!scan(&dirs, &["Meridian"]).iter().any(|e| e.item.id == "example.desktop"));
        std::fs::write(dirs[1].join("excluded.desktop"), format!("{source}NotShowIn=Meridian;\n")).unwrap();
        std::fs::write(dirs[1].join("disabled.desktop"), format!("{source}X-GNOME-Autostart-enabled=false\n")).unwrap();
        let entries = scan(&dirs, &["Meridian"]);
        assert!(!entries.iter().find(|e| e.item.id == "excluded.desktop").unwrap().item.available);
        assert!(!entries.iter().find(|e| e.item.id == "disabled.desktop").unwrap().item.enabled);
        std::fs::remove_dir_all(root).unwrap();
    }
}
