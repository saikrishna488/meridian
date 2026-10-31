//! Input options for Meridian's labwc session. Do not replace user keybindings.
use meridian_protocol::{ErrorBody, ErrorCode, InputPreferences};
use std::{fs, path::PathBuf, process::Command};

const BEGIN: &str = "<!-- meridian-input-start -->";
const END: &str = "<!-- meridian-input-end -->";

fn invalid(message: &str) -> ErrorBody { ErrorBody::new(ErrorCode::InvalidRequest, message) }
fn failed(message: impl std::fmt::Display) -> ErrorBody { ErrorBody::new(ErrorCode::Failed, message.to_string()) }
fn file() -> PathBuf { glib::user_config_dir().join("labwc/rc.xml") }
fn preferences() -> PathBuf { glib::user_config_dir().join("meridian/input-preferences.json") }

pub fn get() -> Result<InputPreferences, ErrorBody> {
    Ok(crate::config::read::<InputPreferences>(&preferences()).unwrap_or_default())
}
fn yes(value: bool) -> &'static str { if value { "yes" } else { "no" } }
fn validate(value: &InputPreferences) -> Result<(), ErrorBody> {
    if !(1..=100).contains(&value.repeat_rate) || !(100..=2000).contains(&value.repeat_delay) {
        return Err(invalid("Keyboard repeat rate must be 1–100 and delay must be 100–2000 ms"));
    }
    if !value.mouse_speed.is_finite() || !(-1.0..=1.0).contains(&value.mouse_speed)
        || !value.touchpad_speed.is_finite() || !(-1.0..=1.0).contains(&value.touchpad_speed)
    { return Err(invalid("Pointer speed must be between -1 and 1")); }
    Ok(())
}
fn input_block(value: &InputPreferences) -> String {
    format!(r#"{BEGIN}
  <libinput>
    <device category="non-touch">
      <pointerSpeed>{:.2}</pointerSpeed>
      <accelProfile>{}</accelProfile>
      <naturalScroll>{}</naturalScroll>
      <leftHanded>{}</leftHanded>
    </device>
    <device category="touchpad">
      <pointerSpeed>{:.2}</pointerSpeed>
      <tap>{}</tap>
      <tapAndDrag>{}</tapAndDrag>
      <naturalScroll>{}</naturalScroll>
      <disableWhileTyping>{}</disableWhileTyping>
      <scrollMethod>twoFinger</scrollMethod>
    </device>
  </libinput>
  {END}"#,
      value.mouse_speed, if value.mouse_acceleration { "adaptive" } else { "flat" },
      yes(value.mouse_natural_scroll), yes(value.mouse_left_handed),
      value.touchpad_speed, yes(value.tap_to_click), yes(value.tap_and_drag),
      yes(value.touchpad_natural_scroll), yes(value.disable_while_typing))
}
fn update_xml(xml: &str, value: &InputPreferences) -> Result<String, ErrorBody> {
    if !xml.contains("<labwc_config>") || !xml.contains("</labwc_config>")
        || !xml.contains("<keyboard>") || !xml.contains("</keyboard>") {
        return Err(failed("labwc rc.xml does not contain expected configuration sections"));
    }
    let mut edited = xml.to_owned();
    if let Some(start) = edited.find(BEGIN) {
        let end = edited.find(END).ok_or_else(|| failed("Meridian input section is incomplete"))? + END.len();
        if end < start { return Err(failed("Meridian input section markers are out of order")); }
        // Remove the entire generated line range, not only the markers;
        // otherwise every save accumulates extra blank lines.
        let line_start = edited[..start].rfind('\n').map_or(0, |index| index + 1);
        let line_end = edited[end..].find('\n').map_or(end, |index| end + index + 1);
        edited.replace_range(line_start..line_end, "");
    }
    // For the first install, refuse to override user-defined input profiles.
    if edited.contains("<libinput>") {
        return Err(failed("Existing custom libinput profiles detected; configure them in labwc rc.xml first"));
    }
    // Override repeat fields inside the existing keyboard section (without altering shortcuts).
    for tag in ["repeatRate", "repeatDelay"] {
        let open = format!("<{tag}>");
        if let Some(start) = edited.find(&open) {
            let close = format!("</{tag}>");
            let end = edited[start..].find(&close).ok_or_else(|| failed("Malformed keyboard repeat setting"))?
                + start + close.len();
            edited.replace_range(start..end, "");
        }
    }
    edited = edited.replacen("</keyboard>", &format!("    <repeatRate>{}</repeatRate>\n    <repeatDelay>{}</repeatDelay>\n  </keyboard>", value.repeat_rate, value.repeat_delay), 1);
    Ok(edited.replacen("</labwc_config>", &format!("{}\n</labwc_config>", input_block(value)), 1))
}

pub fn set(value: InputPreferences) -> Result<InputPreferences, ErrorBody> {
    validate(&value)?;
    if std::env::var("LABWC_PID").ok().filter(|pid| pid.parse::<u32>().ok().filter(|n| *n > 0).is_some()).is_none() {
        return Err(ErrorBody::new(ErrorCode::Unavailable, "Input controls require an active labwc session"));
    }
    let path = file();
    let original = fs::read_to_string(&path)
        .map_err(|_| ErrorBody::new(ErrorCode::Unavailable, "labwc rc.xml is not installed for this session"))?;
    let edited = update_xml(&original, &value)?;
    // Preserve original user configuration in case they need to undo the edit.
    let backup = path.with_extension("xml.meridian-backup");
    if !backup.exists() { fs::write(&backup, &original).map_err(failed)?; }
    let temporary = path.with_extension("xml.meridian-new");
    fs::write(&temporary, &edited).map_err(failed)?;
    fs::rename(&temporary, &path).map_err(failed)?;
    let result = Command::new("labwc").arg("--reconfigure").status();
    if !result.is_ok_and(|status| status.success()) {
        fs::write(&path, original).map_err(failed)?;
        return Err(failed("labwc could not reload input settings; original configuration restored"));
    }
    crate::config::save(&preferences(), &value)?;
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_input_without_replacing_keybindings() {
        let base = r#"<labwc_config><keyboard><keybind key="W-e" /></keyboard></labwc_config>"#;
        let prefs = InputPreferences::default();
        let result = update_xml(base, &prefs).unwrap();
        assert!(result.contains("keybind key=\"W-e\""));
        assert!(result.contains("<repeatRate>25</repeatRate>"));
        assert!(result.contains("<device category=\"touchpad\">"));
        assert!(update_xml(&result, &prefs).is_ok());
        assert_eq!(result, update_xml(&result, &prefs).unwrap());
    }
    #[test]
    fn rejects_existing_custom_libinput() {
        let xml = "<labwc_config><keyboard></keyboard><libinput></libinput></labwc_config>";
        assert!(update_xml(xml, &InputPreferences::default()).is_err());
    }
    #[test]
    fn validates_speed_and_repeat_ranges() {
        let mut prefs = InputPreferences::default();
        prefs.mouse_speed = 2.0;
        assert!(validate(&prefs).is_err());
        prefs.mouse_speed = 0.0;
        prefs.repeat_delay = 0;
        assert!(validate(&prefs).is_err());
    }
}
