//! Persistent wallpaper choices and shortcuts. Removing a shortcut never
//! touches its target. Custom wallpaper copies remain available if moved.
use gtk::glib;
use meridian_protocol::{DesktopItem, DesktopState, ErrorBody, ErrorCode, WallpaperChoice};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

static CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const WALLPAPERS: [(&str, &str); 4] = [
    ("meridian-dusk", "Dusk"),
    ("meridian-tide", "Tide"),
    ("meridian-alpine", "Alpine"),
    ("meridian-aurora", "Aurora"),
];
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Config {
    appearance: String,
    wallpaper: String,
    custom: Option<String>,
    revision: u64,
    items: Vec<DesktopItem>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            appearance: "light".into(),
            wallpaper: "meridian-dusk".into(),
            custom: None,
            revision: 0,
            items: Vec::new(),
        }
    }
}
fn dir() -> PathBuf {
    glib::user_config_dir().join("meridian")
}
fn load() -> Config {
    load_at(&dir())
}
fn load_at(directory: &Path) -> Config {
    std::fs::read(directory.join("desktop.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
fn save(config: &Config) -> Result<(), ErrorBody> {
    save_at(&dir(), config)
}
fn save_at(directory: &Path, config: &Config) -> Result<(), ErrorBody> {
    std::fs::create_dir_all(directory).map_err(error)?;
    let temp = directory.join("desktop.json.tmp");
    std::fs::write(&temp, serde_json::to_vec_pretty(config).map_err(error)?).map_err(error)?;
    std::fs::rename(temp, directory.join("desktop.json")).map_err(error)
}
fn state(config: &Config) -> DesktopState {
    let wallpapers = WALLPAPERS
        .iter()
        .map(|(id, name)| WallpaperChoice {
            id: (*id).into(),
            name: (*name).into(),
            uri: format!("meridian://assets/wallpapers/{id}.svg"),
        })
        .collect();
    let wallpaper_uri = if config.wallpaper == "custom" && config.custom.is_some() {
        format!("meridian://desktop/wallpaper?v={}", config.revision)
    } else {
        let id =
            if WALLPAPERS.iter().any(|(id, _)| *id == config.wallpaper) { &config.wallpaper } else { "meridian-dusk" };
        format!("meridian://assets/wallpapers/{id}.svg")
    };
    DesktopState {
        appearance: config.appearance.clone(),
        wallpaper: config.wallpaper.clone(),
        wallpaper_uri,
        wallpapers,
        items: config.items.clone(),
    }
}
pub fn get() -> DesktopState {
    state(&load())
}
pub fn set_wallpaper(id: &str) -> Result<DesktopState, ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    if !WALLPAPERS.iter().any(|(candidate, _)| *candidate == id) {
        return Err(error("Unknown wallpaper"));
    }
    let mut config = load();
    config.wallpaper = id.into();
    save(&config)?;
    Ok(state(&config))
}
pub fn custom_wallpaper() -> Option<PathBuf> {
    let name = load().custom?;
    if !matches!(name.as_str(), "wallpaper.png" | "wallpaper.jpg" | "wallpaper.webp") {
        return None;
    }
    Some(dir().join(name))
}
pub fn choose_wallpaper(path: &Path) -> Result<DesktopState, ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    if std::fs::metadata(path).map_err(error)?.len() > 20 * 1024 * 1024 {
        return Err(error("Choose a wallpaper smaller than 20 MB."));
    }
    let bytes = std::fs::read(path).map_err(error)?;
    let extension = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "jpg"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "webp"
    } else {
        return Err(error("Choose a PNG, JPEG, or WebP image."));
    };
    let name = format!("wallpaper.{extension}");
    std::fs::create_dir_all(dir()).map_err(error)?;
    std::fs::write(dir().join(&name), bytes).map_err(error)?;
    let mut config = load();
    config.wallpaper = "custom".into();
    config.custom = Some(name);
    config.revision = config.revision.wrapping_add(1);
    save(&config)?;
    Ok(state(&config))
}
pub fn add(item: DesktopItem) -> Result<DesktopState, ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    let mut config = load();
    if !config.items.iter().any(|i| i.kind == item.kind && i.target == item.target) {
        if config.items.len() >= 200 {
            return Err(error("The desktop already has 200 shortcuts."));
        }
        config.items.push(item);
    }
    save(&config)?;
    Ok(state(&config))
}
pub fn add_file(path: &str) -> Result<DesktopState, ErrorBody> {
    let path = Path::new(path).canonicalize().map_err(error)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
    let target = path.to_str().ok_or_else(|| error("Unsupported filename"))?.to_owned();
    add(DesktopItem {
        id: format!("file:{target}"),
        name,
        kind: if path.is_dir() { "folder" } else { "file" }.into(),
        target,
        icon: None,
    })
}
pub fn remove(id: &str) -> Result<DesktopState, ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    remove_at(&dir(), id)
}
fn remove_at(directory: &Path, id: &str) -> Result<DesktopState, ErrorBody> {
    let mut config = load_at(directory);
    config.items.retain(|i| i.id != id);
    save_at(directory, &config)?;
    Ok(state(&config))
}
pub fn item(id: &str) -> Result<DesktopItem, ErrorBody> {
    load().items.into_iter().find(|i| i.id == id).ok_or_else(|| error("This desktop shortcut no longer exists."))
}
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}

/// Save the preference and update GTK and portal-aware applications via GSettings.
pub fn set_appearance(mode: &str) -> Result<DesktopState, ErrorBody> {
    if !matches!(mode, "light" | "dark") {
        return Err(error("Unknown appearance"));
    }
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    let mut config = load();
    config.appearance = mode.into();
    save(&config)?;
    // GTK applications that do not read the portal also inherit the saved preference.
    for version in ["gtk-3.0", "gtk-4.0"] {
        let directory = glib::user_config_dir().join(version);
        std::fs::create_dir_all(&directory).map_err(error)?;
        let path = directory.join("settings.ini");
        let file = glib::KeyFile::new();
        if path.exists() {
            file.load_from_file(&path, glib::KeyFileFlags::KEEP_COMMENTS).map_err(error)?;
        }
        file.set_boolean("Settings", "gtk-application-prefer-dark-theme", mode == "dark");
        file.set_string("Settings", "gtk-theme-name", if mode == "dark" { "Adwaita-dark" } else { "Adwaita" });
        std::fs::write(&path, file.to_data()).map_err(error)?;
    }
    use gtk::gio::prelude::*;
    if let Some(schema) =
        gtk::gio::SettingsSchemaSource::default().and_then(|source| source.lookup("org.gnome.desktop.interface", true))
    {
        let settings = gtk::gio::Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
        if schema.has_key("color-scheme") {
            settings
                .set_string("color-scheme", if mode == "dark" { "prefer-dark" } else { "prefer-light" })
                .map_err(error)?;
        }
        if schema.has_key("gtk-theme") {
            settings.set_string("gtk-theme", if mode == "dark" { "Adwaita-dark" } else { "Adwaita" }).map_err(error)?;
        }
        gtk::gio::Settings::sync();
    }
    // KDE applications follow their shared palette without changing icons.
    let kde = if std::path::Path::new("/usr/bin/plasma-apply-colorscheme").exists() {
        Some("/usr/bin/plasma-apply-colorscheme")
    } else {
        None
    };
    if let Some(command) = kde {
        let result = std::process::Command::new(command)
            .arg(if mode == "dark" { "BreezeDark" } else { "BreezeLight" })
            .output()
            .map_err(error)?;
        if !result.status.success() {
            return Err(error("Appearance was saved, but the KDE palette could not be updated"));
        }
    }
    Ok(state(&config))
}

/// A free SF-like UI typeface shared by native and web windows.
pub fn apply_fonts() -> Result<(), ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    use gtk::gio::prelude::*;
    for version in ["gtk-3.0", "gtk-4.0"] {
        let directory = glib::user_config_dir().join(version);
        std::fs::create_dir_all(&directory).map_err(error)?;
        let path = directory.join("settings.ini");
        let file = glib::KeyFile::new();
        if path.exists() {
            file.load_from_file(&path, glib::KeyFileFlags::KEEP_COMMENTS).map_err(error)?;
        }
        file.set_string("Settings", "gtk-font-name", "Inter 11");
        std::fs::write(&path, file.to_data()).map_err(error)?;
    }
    if let Some(schema) =
        gtk::gio::SettingsSchemaSource::default().and_then(|source| source.lookup("org.gnome.desktop.interface", true))
    {
        let settings = gtk::gio::Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
        if schema.has_key("font-name") {
            settings.set_string("font-name", "Inter 11").map_err(error)?;
        }
        gtk::gio::Settings::sync();
    }
    let path = glib::user_config_dir().join("kdeglobals");
    let file = glib::KeyFile::new();
    if path.exists() {
        file.load_from_file(&path, glib::KeyFileFlags::KEEP_COMMENTS).map_err(error)?;
    }
    file.set_string("General", "font", "Inter,10,-1,5,50,0,0,0,0,0");
    for key in ["menuFont", "toolBarFont", "taskbarFont"] {
        file.set_string("General", key, "Inter,10,-1,5,50,0,0,0,0,0");
    }
    file.set_string("General", "smallestReadableFont", "Inter,9,-1,5,50,0,0,0,0,0");
    std::fs::write(path, file.to_data()).map_err(error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wallpaper_and_shortcuts_persist_and_removing_shortcut_keeps_target() {
        let directory = std::env::temp_dir().join(format!("meridian-desktop-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let target = directory.join("original.txt");
        std::fs::write(&target, "keep me").unwrap();
        let config = Config {
            appearance: "dark".into(),
            wallpaper: "meridian-tide".into(),
            items: vec![DesktopItem {
                id: "shortcut".into(),
                name: "original.txt".into(),
                kind: "file".into(),
                target: target.to_string_lossy().into_owned(),
                icon: None,
            }],
            ..Config::default()
        };
        save_at(&directory, &config).unwrap();
        let loaded = load_at(&directory);
        assert_eq!(loaded.wallpaper, "meridian-tide");
        assert_eq!(state(&loaded).appearance, "dark");
        assert_eq!(loaded.items.len(), 1);
        assert!(state(&loaded).wallpaper_uri.ends_with("meridian-tide.svg"));
        assert!(remove_at(&directory, "shortcut").unwrap().items.is_empty());
        assert!(load_at(&directory).items.is_empty());
        assert_eq!(std::fs::read_to_string(target).unwrap(), "keep me");
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn invalid_wallpaper_id_cannot_become_a_filesystem_path() {
        let config = Config { wallpaper: "../../etc/passwd".into(), ..Config::default() };
        assert!(state(&config).wallpaper_uri.ends_with("meridian-dusk.svg"));
    }
}
