//! Small, private desktop preferences and managed compositor shortcuts.
use gtk::{gio, prelude::*};
use meridian_protocol::{ErrorBody, ErrorCode, ShortcutEntry, WorkflowChange, WorkflowState};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub shortcuts: BTreeMap<String, String>,
    pub completed: Vec<String>,
    pub text_scale: f64,
    pub high_contrast: bool,
    pub reduced_motion: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            shortcuts: BTreeMap::new(),
            completed: vec![],
            text_scale: 1.0,
            high_contrast: false,
            reduced_motion: false,
        }
    }
}
const SHORTCUTS: &[(&str, &str, &str)] = &[
    ("applications", "Applications", "W-space"),
    ("controls", "Control Center", "W-c"),
    ("files", "Files", "W-e"),
    ("settings", "Settings", "W-i"),
    ("terminal", "Terminal", "W-Return"),
    ("lock", "Lock session", "W-l"),
    ("desktop", "Show desktop", "W-d"),
    ("screenshot", "Screenshot", "Print"),
    ("region", "Screenshot area", "W-S-s"),
];
fn error(message: impl ToString) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, message.to_string())
}
fn path() -> PathBuf {
    glib::user_config_dir().join("meridian/workflows.json")
}
pub fn preferences() -> Preferences {
    let mut p = crate::config::read_versioned::<Preferences>(&path()).unwrap_or_default();
    if !p.text_scale.is_finite() || !(1.0..=1.5).contains(&p.text_scale) {
        p.text_scale = 1.0;
    }
    p
}
pub fn take_first_run() -> bool {
    let mut p = preferences();
    if p.completed.iter().any(|id| id == "presented" || id == "dismissed") {
        return false;
    }
    p.completed.push("presented".into());
    crate::config::save_versioned(&path(), &p).is_ok()
}
pub fn state() -> WorkflowState {
    let p = preferences();
    WorkflowState {
        shortcuts: SHORTCUTS
            .iter()
            .map(|(id, label, key)| ShortcutEntry {
                id: (*id).into(),
                label: (*label).into(),
                key: p.shortcuts.get(*id).cloned().unwrap_or_else(|| (*key).into()),
                default_key: (*key).into(),
            })
            .collect(),
        completed: p.completed,
        text_scale: p.text_scale,
        high_contrast: p.high_contrast,
        reduced_motion: p.reduced_motion,
        surfaces: vec![],
        errors: recent_errors(),
    }
}
thread_local! { static ERRORS: std::cell::RefCell<std::collections::VecDeque<String>> = Default::default(); }
pub fn record_error(message: String) {
    ERRORS.with(|errors| {
        let mut errors = errors.borrow_mut();
        if errors.len() >= 30 {
            errors.pop_front();
        }
        errors.push_back(message);
    });
}
pub fn recent_errors() -> Vec<String> {
    ERRORS.with(|e| e.borrow().iter().rev().cloned().collect())
}
fn write(path: &Path, data: &[u8]) -> Result<(), ErrorBody> {
    gio::File::for_path(path)
        .replace_contents(
            data,
            None,
            false,
            gio::FileCreateFlags::PRIVATE | gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::Cancellable::NONE,
        )
        .map_err(error)?;
    Ok(())
}
fn normalized_key(key: &str) -> Result<String, ErrorBody> {
    let mut parts: Vec<_> = key.split('-').collect();
    let symbol = parts.pop().unwrap_or_default();
    let valid = (symbol.len() == 1 && symbol.bytes().all(|b| b.is_ascii_alphanumeric()))
        || [
            "space",
            "Return",
            "Tab",
            "Escape",
            "Print",
            "BackSpace",
            "Delete",
            "Insert",
            "Home",
            "End",
            "Left",
            "Right",
            "Up",
            "Down",
            "Page_Up",
            "Page_Down",
            "Super_L",
        ]
        .contains(&symbol)
        || symbol.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=24).contains(&n));
    if !valid || parts.iter().any(|p| !["W", "C", "A", "S"].contains(p)) {
        return Err(error("Choose a letter, digit, function key or named navigation key with W/C/A/S modifiers."));
    }
    if symbol.len() == 1 && !parts.iter().any(|p| ["W", "C", "A"].contains(p)) {
        return Err(error("Use Super, Control or Alt with letters and digits so normal typing is preserved."));
    }
    let mut modifiers = parts.clone();
    modifiers.sort();
    modifiers.dedup();
    if modifiers.len() != parts.len() {
        return Err(error("A modifier is repeated"));
    }
    modifiers.push(symbol);
    Ok(modifiers.join("-").to_ascii_lowercase())
}
fn render(base: &str, p: &Preferences) -> Result<String, ErrorBody> {
    // Only replace the key attribute of known bundled bindings; never insert commands.
    let mut result = base.to_owned();
    let mut keys: Vec<String> =
        base.split("<keybind key=\"").skip(1).filter_map(|s| s.split('"').next().map(str::to_owned)).collect();
    for (_, _, key) in SHORTCUTS {
        keys.retain(|k| k != key);
    }
    // These also come from labwc's <default/>; avoid silently shadowing them.
    keys.extend(["A-Tab", "A-S-Tab", "A-F4", "A-F2", "W-Tab", "C-A-Delete"].map(str::to_owned));
    for (id, _, default) in SHORTCUTS {
        let key = p.shortcuts.get(*id).map(String::as_str).unwrap_or(default);
        if key.is_empty() || key.len() > 64 || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(error("Use labwc key notation, for example W-S-e or C-A-Return."));
        }
        normalized_key(key)?;
        if keys.iter().any(|k| normalized_key(k).ok().as_ref() == normalized_key(key).ok().as_ref()) {
            return Err(error(format!("Shortcut {key} is already assigned.")));
        }
        keys.push(key.into());
        let needle = format!("<keybind key=\"{default}\"");
        if base.matches(&needle).count() != 1 {
            return Err(error(
                "The session configuration has changed. Restore its managed bindings before editing shortcuts.",
            ));
        }
        // Placeholders avoid swapping keys changing another binding twice.
        result = result.replace(&needle, &format!("<keybind key=\"MERIDIAN_PLACEHOLDER_{id}\""));
    }
    for (id, _, default) in SHORTCUTS {
        result = result.replace(
            &format!("MERIDIAN_PLACEHOLDER_{id}"),
            p.shortcuts.get(*id).map(String::as_str).unwrap_or(default),
        );
    }
    Ok(result)
}
pub fn prepare_session(base: &Path) -> Result<PathBuf, ErrorBody> {
    let directory = glib::user_runtime_dir().join("meridian-labwc");
    std::fs::create_dir_all(&directory).map_err(error)?;
    let text = std::fs::read_to_string(base.join("rc.xml")).map_err(error)?;
    let rendered = render(&text, &preferences()).unwrap_or_else(|error| {
        eprintln!("Using base shortcuts: {}", error.message);
        text.clone()
    });
    write(&directory.join("rc.xml"), rendered.as_bytes())?;
    // Preserve menu, autostart and environment supplied by the selected session.
    for name in ["menu.xml", "autostart", "environment", "themerc", "idle", "polkit-agent"] {
        let target = directory.join(name);
        if target.symlink_metadata().is_ok() {
            std::fs::remove_file(&target).map_err(error)?;
        }
        if base.join(name).exists() {
            std::os::unix::fs::symlink(base.join(name), target).map_err(error)?;
        }
    }
    Ok(directory)
}
pub fn change(change: WorkflowChange) -> Result<(), ErrorBody> {
    let mut p = preferences();
    match change.action.as_str() {
        "shortcut" => {
            if !SHORTCUTS.iter().any(|(id, _, _)| *id == change.id) {
                return Err(error("Unknown shortcut"));
            }
            if change.value.is_empty() {
                p.shortcuts.remove(&change.id);
            } else {
                p.shortcuts.insert(change.id, change.value);
            }
            let base = std::env::var_os("MERIDIAN_LABWC_BASE").ok_or_else(|| {
                error("Editable shortcuts require a Meridian session started with the updated session launcher.")
            })?;
            let active = std::env::var_os("MERIDIAN_LABWC_CONFIG")
                .ok_or_else(|| error("Managed session configuration unavailable"))?;
            let text = std::fs::read_to_string(Path::new(&base).join("rc.xml")).map_err(error)?;
            let rendered = render(&text, &p)?;
            let active = Path::new(&active).join("rc.xml");
            let before = std::fs::read(&active).map_err(error)?;
            write(&active, rendered.as_bytes())?;
            if let Err(e) = crate::config::save_versioned(&path(), &p) {
                let _ = write(&active, &before);
                return Err(e);
            }
            // Saved state is authoritative; a failed reload is reported rather than hidden.
            crate::settings_devices::run_timeout(&["labwc", "--reconfigure"], None, std::time::Duration::from_secs(3))
                .map_err(|_| error("Shortcut saved. Sign out and back in to apply; compositor reload failed."))?;
            return Ok(());
        }
        "checklist" => {
            if !["displays", "keyboard", "sound", "applications", "dismissed"].contains(&change.id.as_str()) {
                return Err(error("Unknown setup item"));
            }
            p.completed.retain(|id| id != &change.id);
            if change.value == "true" {
                p.completed.push(change.id);
            }
        }
        "text-scale" => {
            p.text_scale = change.value.parse().map_err(error)?;
            if !p.text_scale.is_finite() || !(1.0..=1.5).contains(&p.text_scale) {
                return Err(error("Text scale must be between 1 and 1.5"));
            }
        }
        "high-contrast" => p.high_contrast = change.value == "true",
        "reduced-motion" => p.reduced_motion = change.value == "true",
        _ => return Err(error("Unknown desktop preference")),
    }
    crate::config::save_versioned(&path(), &p)
}
