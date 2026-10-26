//! Wallpaper preferences, application shortcuts, and the real XDG Desktop folder.
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
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    primary_display: Option<String>,
    appearance: String,
    font: String,
    wallpaper: String,
    custom: Option<String>,
    revision: u64,
    items: Vec<DesktopItem>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            primary_display: None,
            appearance: "dark".into(),
            font: "Inter".into(),
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
    crate::config::read_versioned(&directory.join("desktop.json")).unwrap_or_default()
}
fn save(config: &Config) -> Result<(), ErrorBody> {
    save_at(&dir(), config)
}
fn save_at(directory: &Path, config: &Config) -> Result<(), ErrorBody> {
    crate::config::save_versioned(&directory.join("desktop.json"), config)
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
        font: config.font.clone(),
        wallpaper: config.wallpaper.clone(),
        wallpaper_uri,
        wallpapers,
        items: desktop_items(config),
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
    if !valid_custom_name(&name) {
        return None;
    }
    Some(dir().join(name))
}
fn valid_custom_name(name: &str) -> bool {
    ["png", "jpg", "webp"].iter().any(|extension| {
        name.strip_prefix("wallpaper-")
            .and_then(|rest| rest.strip_suffix(&format!(".{extension}")))
            .is_some_and(|revision| !revision.is_empty() && revision.bytes().all(|byte| byte.is_ascii_digit()))
    }) || matches!(name, "wallpaper.png" | "wallpaper.jpg" | "wallpaper.webp")
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
    std::fs::create_dir_all(dir()).map_err(error)?;
    let mut config = load();
    config.revision = config.revision.wrapping_add(1);
    let name = format!("wallpaper-{}.{extension}", config.revision);
    let directory = dir();
    let temporary = directory.join(format!(".{name}.tmp"));
    let target = directory.join(&name);
    let _ = std::fs::remove_file(&temporary);
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(error)?;
        file.write_all(&bytes).map_err(error)?;
        file.sync_all().map_err(error)?;
    }
    if let Err(failure) = std::fs::rename(&temporary, &target) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error(failure));
    }
    config.wallpaper = "custom".into();
    config.custom = Some(name);
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
pub fn folder() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Desktop)
        .filter(|path| path != &glib::home_dir())
        .unwrap_or_else(|| glib::home_dir().join("Desktop"))
}
fn file_item(path: &Path) -> Option<DesktopItem> {
    let target = path.to_str()?.to_owned();
    Some(DesktopItem {
        id: format!("file:{target}"),
        name: path.file_name()?.to_str()?.to_owned(),
        kind: if path.is_dir() { "folder" } else { "file" }.into(),
        target,
        icon: None,
    })
}
fn desktop_items(config: &Config) -> Vec<DesktopItem> {
    // Retain old shortcuts without duplicating files already present on Desktop.
    let mut items: Vec<_> = config
        .items
        .iter()
        .cloned()
        .map(|mut item| {
            if item.kind == "app" && item.target == "meridian-finder.desktop" {
                item.name = "File Manager".into();
            }
            if item.kind != "app" {
                item.id = format!("shortcut:{}", item.id);
            }
            item
        })
        .collect();
    if let Ok(entries) = std::fs::read_dir(folder()) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if let Some(item) = file_item(&entry.path()) {
                items.retain(|old| old.target != item.target);
                items.push(item);
            }
        }
    }
    items
}
fn move_to_folder(source: &Path, destination: &Path) -> Result<(), ErrorBody> {
    let source = crate::files::absolute(source.to_str().ok_or_else(|| error("Choose a UTF-8 file path."))?)?;
    let parent = source.parent().ok_or_else(|| error("Cannot move this location"))?.canonicalize().map_err(error)?;
    std::fs::create_dir_all(destination).map_err(error)?;
    let destination = destination.canonicalize().map_err(error)?;
    if parent == destination {
        return Ok(());
    }
    crate::files::transfer(
        source.to_str().unwrap(),
        destination.to_str().ok_or_else(|| error("Choose a UTF-8 folder path."))?,
        true,
    )
}

pub fn add_file(path: &str) -> Result<DesktopState, ErrorBody> {
    move_to_folder(Path::new(path), &folder())?;
    Ok(get())
}
pub fn remove(id: &str) -> Result<DesktopState, ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    let mut config = load();
    let stored_id = id.strip_prefix("shortcut:").unwrap_or(id);
    if id.starts_with("file:") {
        let entry = item(id)?;
        crate::files::trash(&entry.target)?;
        config.items.retain(|item| item.target != entry.target);
        save(&config)?;
        return Ok(state(&config));
    }
    if !config.items.iter().any(|item| item.id == stored_id) {
        return Err(error("This desktop shortcut no longer exists."));
    }
    remove_at(&dir(), stored_id)
}
fn remove_at(directory: &Path, id: &str) -> Result<DesktopState, ErrorBody> {
    let mut config = load_at(directory);
    config.items.retain(|i| i.id != id);
    save_at(directory, &config)?;
    Ok(state(&config))
}
pub fn item(id: &str) -> Result<DesktopItem, ErrorBody> {
    desktop_items(&load()).into_iter().find(|i| i.id == id).ok_or_else(|| error("This desktop item no longer exists."))
}
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}

/// Save the preference and update GTK and portal-aware applications via GSettings.
pub fn set_appearance(mode: &str) -> Result<DesktopState, ErrorBody> {
    if mode != "dark" {
        return Err(error("Unknown appearance"));
    }
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    let mut config = load();
    config.appearance = "dark".into();
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
            return Err(error("The KDE palette could not be updated"));
        }
    }
    save(&config)?;
    Ok(state(&config))
}

/// Shared native typography and complete GTK titlebar controls.
pub fn apply_fonts() -> Result<(), ErrorBody> {
    let font = get().font;
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
        file.set_string("Settings", "gtk-font-name", &format!("{font} 11"));
        file.set_string("Settings", "gtk-decoration-layout", ":minimize,maximize,close");
        std::fs::write(&path, file.to_data()).map_err(error)?;
    }
    if let Some(schema) =
        gtk::gio::SettingsSchemaSource::default().and_then(|source| source.lookup("org.gnome.desktop.interface", true))
    {
        let settings = gtk::gio::Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
        if schema.has_key("font-name") {
            settings.set_string("font-name", &format!("{font} 11")).map_err(error)?;
        }
        gtk::gio::Settings::sync();
    }
    // GTK/libadwaita clients can read the desktop preference instead of settings.ini.
    if let Some(schema) = gtk::gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup("org.gnome.desktop.wm.preferences", true))
    {
        let settings = gtk::gio::Settings::new_full(&schema, None::<&gtk::gio::SettingsBackend>, None);
        if schema.has_key("button-layout") {
            settings.set_string("button-layout", ":minimize,maximize,close").map_err(error)?;
        }
        gtk::gio::Settings::sync();
    }
    let path = glib::user_config_dir().join("kdeglobals");
    let file = glib::KeyFile::new();
    if path.exists() {
        file.load_from_file(&path, glib::KeyFileFlags::KEEP_COMMENTS).map_err(error)?;
    }
    file.set_string("General", "font", &format!("{font},10,-1,5,50,0,0,0,0,0"));
    for key in ["menuFont", "toolBarFont", "taskbarFont"] {
        file.set_string("General", key, &format!("{font},10,-1,5,50,0,0,0,0,0"));
    }
    file.set_string("General", "smallestReadableFont", &format!("{font},9,-1,5,50,0,0,0,0,0"));
    std::fs::write(path, file.to_data()).map_err(error)?;
    Ok(())
}

pub fn set_font(font: &str) -> Result<DesktopState, ErrorBody> {
    if !["Inter", "Noto Sans", "DejaVu Sans", "Liberation Sans"].contains(&font) {
        return Err(error("Unknown font"));
    }
    {
        let _guard = CONFIG_LOCK.lock().map_err(error)?;
        let mut config = load();
        config.font = font.into();
        save(&config)?;
    }
    if let Err(error) = apply_fonts() {
        log::warn!("Could not apply font to external applications: {}", error.message);
    }
    Ok(get())
}

/// Saved connector preference survives unplugging; the shell chooses a fallback.
pub fn primary_display() -> Option<String> {
    load().primary_display
}
pub fn set_primary_display(connector: Option<String>) -> Result<(), ErrorBody> {
    let _guard = CONFIG_LOCK.lock().map_err(error)?;
    let mut config = load();
    config.primary_display = connector;
    save(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_moves_files_without_overwriting() {
        let root = std::env::temp_dir().join(format!("meridian-desktop-move-{}", std::process::id()));
        let desktop = root.join("Desktop");
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("notes.txt");
        std::fs::write(&source, "first").unwrap();
        move_to_folder(&source, &desktop).unwrap();
        assert!(!source.exists());
        assert_eq!(std::fs::read_to_string(desktop.join("notes.txt")).unwrap(), "first");
        std::fs::write(&source, "second").unwrap();
        assert!(move_to_folder(&source, &desktop).is_err());
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "second");
        assert_eq!(std::fs::read_to_string(desktop.join("notes.txt")).unwrap(), "first");
        assert!(move_to_folder(&root, &desktop).is_err());
        move_to_folder(&desktop.join("notes.txt"), &desktop).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wallpaper_and_shortcuts_persist_and_removing_shortcut_keeps_target() {
        let directory = std::env::temp_dir().join(format!("meridian-desktop-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let target = directory.join("original.txt");
        std::fs::write(&target, "keep me").unwrap();
        let config = Config {
            appearance: "dark".into(),
            font: "Noto Sans".into(),
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
        assert_eq!(state(&loaded).font, "Noto Sans");
        let old: Config = serde_json::from_str(r#"{"appearance":"dark"}"#).unwrap();
        assert_eq!(old.font, "Inter");
        assert!(set_font("unknown").is_err());
        assert_eq!(state(&loaded).appearance, "dark");
        assert_eq!(loaded.items.len(), 1);
        assert!(state(&loaded).wallpaper_uri.ends_with("meridian-tide.svg"));
        assert!(!remove_at(&directory, "shortcut").unwrap().items.iter().any(|item| item.id == "shortcut"));
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
