//! User-selectable default applications. GIO owns standard MIME/scheme defaults;
//! Meridian owns only shell-specific Files and Terminal preferences.
use gtk::{gio, glib};
use gio_unix::DesktopAppInfo;
use gtk::prelude::*;
use meridian_protocol::{DefaultAppChoice, DefaultAppSetting, ErrorBody, ErrorCode};
use meridian_services::apps::AppCatalog;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const PURPOSES: &[(&str, &str)] = &[
    ("browser", "x-scheme-handler/https"),
    ("mail", "x-scheme-handler/mailto"),
    ("pdf", "application/pdf"),
    ("images", "image/png"),
    ("text", "text/plain"),
];
const SHELL_PURPOSES: &[&str] = &["files", "terminal"];

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ShellDefaults {
    files: Option<String>,
    terminal: Option<String>,
}
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn path() -> PathBuf { glib::user_config_dir().join("meridian/default-apps.json") }
fn read() -> ShellDefaults { crate::config::read::<ShellDefaults>(&path()).unwrap_or_default() }

pub fn preferred(purpose: &str) -> Option<String> {
    let config = read();
    match purpose {
        "files" => config.files.or_else(|| Some("meridian-finder.desktop".into())),
        "terminal" => config.terminal.filter(|id| id != "meridian-terminal.desktop").or_else(|| Some("kitty.desktop".into())),
        _ => None,
    }
}

pub fn list() -> Vec<DefaultAppSetting> {
    let mut entries = vec![
        DefaultAppSetting { purpose: "files".into(), app_id: preferred("files") },
        DefaultAppSetting { purpose: "terminal".into(), app_id: preferred("terminal") },
    ];
    for &(purpose, mime) in PURPOSES {
        entries.push(DefaultAppSetting {
            purpose: purpose.into(),
            app_id: gio::AppInfo::default_for_type(mime, false).and_then(|app| app.id().map(|s| s.to_string())),
        });
    }
    entries
}

pub fn set(choice: &DefaultAppChoice, catalog: &AppCatalog) -> Result<Vec<DefaultAppSetting>, ErrorBody> {
    let _lock = LOCK.lock().map_err(|_| ErrorBody::new(ErrorCode::Failed, "Default app preferences busy"))?;
    let id = choice.app_id.as_str();
    if !catalog.apps().iter().any(|app| app.id == id)
        && id != "meridian-finder.desktop"
    {
        return Err(ErrorBody::new(ErrorCode::NotFound, "Select an installed application"));
    }
    if SHELL_PURPOSES.contains(&choice.purpose.as_str()) {
        if id == "meridian-terminal.desktop"
            || choice.purpose == "terminal" && id == "meridian-finder.desktop"
        {
            return Err(ErrorBody::new(ErrorCode::InvalidRequest, "This application cannot handle that purpose"));
        }
        let mut config = read();
        match choice.purpose.as_str() {
            "files" => config.files = Some(id.into()),
            "terminal" => config.terminal = Some(id.into()),
            _ => unreachable!(),
        }
        crate::config::save(&path(), &config)?;
    } else {
        let mime = PURPOSES.iter().find(|(purpose, _)| *purpose == choice.purpose).map(|(_, mime)| *mime)
            .ok_or_else(|| ErrorBody::new(ErrorCode::InvalidRequest, "Unknown default app purpose"))?;
        let app = DesktopAppInfo::new(id)
            .ok_or_else(|| ErrorBody::new(ErrorCode::NotFound, "Application desktop entry is unavailable"))?;
        app.set_as_default_for_type(mime)
            .map_err(|error| ErrorBody::new(ErrorCode::Failed, format!("Cannot set the default application: {error}")))?;
    }
    Ok(list())
}

pub fn launch_file_manager(path: &str) -> Result<bool, ErrorBody> {
    let Some(id) = preferred("files") else { return Ok(false) };
    if id == "meridian-finder.desktop" { return Ok(false) }
    let app = DesktopAppInfo::new(&id).ok_or_else(|| ErrorBody::new(ErrorCode::NotFound, "Default file manager is unavailable"))?;
    let file = gio::File::for_path(path);
    app.launch(&[file], None::<&gio::AppLaunchContext>)
        .map_err(|error| ErrorBody::new(ErrorCode::Failed, format!("Cannot open file manager: {error}")))?;
    Ok(true)
}

pub fn launch_terminal() -> bool {
    let Some(id) = preferred("terminal") else { return false };
    if id == "kitty.desktop" || id == "net.kovidgoyal.kitty.desktop" { return false }
    let Some(app) = DesktopAppInfo::new(&id) else { return false };
    if let Err(error) = app.launch(&[], None::<&gio::AppLaunchContext>) {
        log::warn!("Default terminal failed to open: {error}");
        return false;
    }
    true
}
