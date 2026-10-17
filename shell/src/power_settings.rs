//! Battery readings and validated screen-lock preferences.
use gtk::{gio, glib, prelude::*};
use meridian_protocol::{BatteryInfo, ErrorBody, ErrorCode, IdleSettings, PowerState};
use std::path::{Path, PathBuf};
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
fn read(path: &Path, key: &str) -> Option<String> {
    std::fs::read_to_string(path.join(key)).ok().map(|s| s.trim().to_owned())
}
fn number(path: &Path, key: &str) -> Option<f64> {
    read(path, key)?.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.)
}
fn ratio(now: f64, full: f64) -> Option<u8> {
    (full > 0.).then(|| (now * 100. / full).round().clamp(0., 100.) as u8)
}
fn status_at(root: &Path) -> PowerState {
    let mut batteries = vec![];
    let mut external = vec![];
    if let Ok(devices) = std::fs::read_dir(root) {
        for path in devices.flatten().map(|e| e.path()) {
            match read(&path, "type").as_deref() {
                Some("Battery")
                    if read(&path, "present").as_deref() != Some("0")
                        && read(&path, "scope").as_deref() != Some("Device") =>
                {
                    let pair = ["energy", "charge"].into_iter().find_map(|prefix| {
                        Some((number(&path, &format!("{prefix}_now"))?, number(&path, &format!("{prefix}_full"))?))
                    });
                    let percent = number(&path, "capacity")
                        .filter(|p| *p <= 100.)
                        .map(|p| p.round() as u8)
                        .or_else(|| pair.and_then(|(n, f)| ratio(n, f)));
                    let health = ["energy", "charge"].into_iter().find_map(|prefix| {
                        ratio(
                            number(&path, &format!("{prefix}_full"))?,
                            number(&path, &format!("{prefix}_full_design"))?,
                        )
                    });
                    batteries.push(BatteryInfo {
                        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                        percent,
                        status: read(&path, "status").unwrap_or_else(|| "Unknown".into()),
                        health,
                    });
                }
                Some("Mains" | "USB" | "USB_C" | "USB_PD" | "USB_PD_DRP" | "Wireless") => {
                    if let Some(online) = read(&path, "online")
                        && (online == "0" || online == "1")
                    {
                        external.push(online == "1");
                    }
                }
                _ => {}
            }
        }
    }
    batteries.sort_by(|a, b| a.name.cmp(&b.name));
    PowerState { on_ac: if external.is_empty() { None } else { Some(external.iter().any(|v| *v)) }, batteries }
}
pub fn status() -> Result<PowerState, ErrorBody> {
    Ok(status_at(Path::new("/sys/class/power_supply")))
}
fn path() -> PathBuf {
    glib::user_config_dir().join("meridian/power.json")
}
pub fn defaults() -> IdleSettings {
    IdleSettings { lock_after: 300, blank_after: 330 }
}
fn valid(settings: &IdleSettings) -> bool {
    [60, 120, 300, 600, 900, 1800].contains(&settings.lock_after)
        && [30, 60, 120, 300].contains(&settings.blank_after.saturating_sub(settings.lock_after))
        && settings.blank_after > settings.lock_after
}
pub fn idle() -> IdleSettings {
    std::fs::read(path())
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .filter(valid)
        .unwrap_or_else(defaults)
}
pub fn save(settings: IdleSettings) -> Result<IdleSettings, ErrorBody> {
    if !valid(&settings) {
        return Err(error("Choose a supported lock delay and screen-off delay."));
    }
    let target = path();
    std::fs::create_dir_all(target.parent().unwrap()).map_err(error)?;
    gio::File::for_path(target)
        .replace_contents(
            &serde_json::to_vec(&settings).map_err(error)?,
            None,
            false,
            gio::FileCreateFlags::PRIVATE | gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::Cancellable::NONE,
        )
        .map_err(error)?;
    Ok(settings)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_battery_and_ac_without_inventing_missing_values() {
        let root = std::env::temp_dir().join(format!("meridian-battery-{}", std::process::id()));
        for (name, values) in [
            (
                "BAT0",
                vec![
                    ("type", "Battery"),
                    ("capacity", "72"),
                    ("status", "Discharging"),
                    ("energy_full", "4000"),
                    ("energy_full_design", "5000"),
                ],
            ),
            ("AC", vec![("type", "Mains"), ("online", "0")]),
            ("mouse", vec![("type", "Battery"), ("scope", "Device"), ("capacity", "50")]),
        ] {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            for (k, v) in values {
                std::fs::write(dir.join(k), v).unwrap();
            }
        }
        let state = status_at(&root);
        assert_eq!(state.on_ac, Some(false));
        assert_eq!(state.batteries.len(), 1);
        assert_eq!(state.batteries[0].percent, Some(72));
        assert_eq!(state.batteries[0].health, Some(80));
        std::fs::remove_file(root.join("BAT0/capacity")).unwrap();
        assert_eq!(status_at(&root).batteries[0].percent, None);
        std::fs::write(root.join("BAT0/present"), "0").unwrap();
        assert!(status_at(&root).batteries.is_empty());
        std::fs::remove_dir_all(root).unwrap();
        assert!(valid(&defaults()));
        assert!(!valid(&IdleSettings { lock_after: 0, blank_after: 330 }));
        assert!(!valid(&IdleSettings { lock_after: 300, blank_after: 299 }));
        assert!(!valid(&IdleSettings { lock_after: 300, blank_after: u32::MAX }));
    }
}
