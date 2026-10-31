//! Input options for Meridian's labwc session. Do not replace user keybindings.
use meridian_protocol::{ErrorBody, ErrorCode, InputPreferences};
use std::{fs, path::PathBuf};

fn invalid(message: &str) -> ErrorBody {
    ErrorBody::new(ErrorCode::InvalidRequest, message)
}
fn preferences() -> PathBuf {
    glib::user_config_dir().join("meridian/input-preferences.json")
}

fn environment_file() -> PathBuf {
    glib::user_config_dir().join("labwc/environment")
}
fn current_layout() -> Option<String> {
    let text = fs::read_to_string(environment_file()).ok()?;
    text.lines().filter_map(|line| line.trim().strip_prefix("XKB_DEFAULT_LAYOUT=")).next_back().map(str::to_owned)
}
pub fn get() -> Result<InputPreferences, ErrorBody> {
    let mut result = crate::config::read::<InputPreferences>(&preferences()).unwrap_or_default();
    if let Some(layout) = current_layout().or_else(|| std::env::var("XKB_DEFAULT_LAYOUT").ok()) {
        result.keyboard_layout = layout;
    }
    Ok(result)
}
fn validate(value: &InputPreferences) -> Result<(), ErrorBody> {
    if !(1..=100).contains(&value.repeat_rate) || !(100..=2000).contains(&value.repeat_delay) {
        return Err(invalid("Keyboard repeat rate must be 1–100 and delay must be 100–2000 ms"));
    }
    if value.keyboard_layout.len() < 2
        || value.keyboard_layout.len() > 3
        || !value.keyboard_layout.bytes().all(|byte| byte.is_ascii_lowercase())
    {
        return Err(invalid("Choose a supported keyboard layout code"));
    }
    if !value.mouse_speed.is_finite()
        || !(-1.0..=1.0).contains(&value.mouse_speed)
        || !value.touchpad_speed.is_finite()
        || !(-1.0..=1.0).contains(&value.touchpad_speed)
    {
        return Err(invalid("Pointer speed must be between -1 and 1"));
    }
    Ok(())
}
pub fn set(value: InputPreferences) -> Result<InputPreferences, ErrorBody> {
    validate(&value)?;
    // labwc does not expose an API to read back per-device libinput settings,
    // and reconfiguration does not apply changes to its startup environment.
    // Do not persist or report a successful change we cannot verify.
    Err(ErrorBody::new(
        ErrorCode::Unavailable,
        "Input changes are unsupported until the running compositor can verify them",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unverifiable_input_changes() {
        let error = set(InputPreferences::default()).unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable);
    }
    #[test]
    fn rejects_invalid_input_values() {
        let mut prefs = InputPreferences { mouse_speed: 2.0, ..InputPreferences::default() };
        assert!(validate(&prefs).is_err());
        prefs.mouse_speed = 0.0;
        prefs.repeat_delay = 0;
        assert!(validate(&prefs).is_err());
    }
}
