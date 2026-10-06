//! Display brightness: read from sysfs, written through systemd-logind
//! (`Session.SetBrightness`), which lets the active session's user change
//! it without root or udev rules.

use std::fs;
use std::path::{Path, PathBuf};

use gio::prelude::*;

const SYSFS: &str = "/sys/class/backlight";

pub struct Backlight {
    name: String,
    dir: PathBuf,
    max: u32,
    session_path: String,
}

impl Backlight {
    /// The preferred backlight device, if the machine has one.
    pub fn find() -> Option<Backlight> {
        let mut candidates: Vec<(u8, PathBuf)> =
            fs::read_dir(SYSFS).ok()?.filter_map(Result::ok).map(|e| e.path()).map(|p| (type_rank(&p), p)).collect();
        // firmware > platform > raw, per the kernel's sysfs-class-backlight docs.
        candidates.sort();
        let dir = candidates.into_iter().next()?.1;
        let max = read_u32(&dir.join("max_brightness")).filter(|&m| m > 0)?;
        let name = dir.file_name()?.to_string_lossy().into_owned();
        log::info!("backlight: {name} (max {max})");
        Some(Backlight { name, dir, max, session_path: session_path() })
    }

    pub fn percent(&self) -> Option<u8> {
        let raw = read_u32(&self.dir.join("brightness"))?;
        Some(to_percent(raw, self.max))
    }

    pub fn set_percent(&self, percent: u8, on_error: impl FnOnce(glib::Error) + 'static) {
        let raw = from_percent(percent, self.max);
        let args = ("backlight", self.name.as_str(), raw).to_variant();
        let Ok(bus) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) else {
            return on_error(glib::Error::new(gio::IOErrorEnum::NotConnected, "no system bus"));
        };
        bus.call(
            Some("org.freedesktop.login1"),
            &self.session_path,
            "org.freedesktop.login1.Session",
            "SetBrightness",
            Some(&args),
            None,
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
            move |result| {
                if let Err(e) = result {
                    on_error(e);
                }
            },
        );
    }
}

fn type_rank(dir: &Path) -> u8 {
    match fs::read_to_string(dir.join("type")).unwrap_or_default().trim() {
        "firmware" => 0,
        "platform" => 1,
        "raw" => 2,
        _ => 3,
    }
}

fn read_u32(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn to_percent(raw: u32, max: u32) -> u8 {
    ((raw.min(max) as f64 / max as f64) * 100.0).round() as u8
}

/// Never go fully dark: 0% maps to the lowest non-zero level.
fn from_percent(percent: u8, max: u32) -> u32 {
    ((percent.min(100) as f64 / 100.0 * max as f64).round() as u32).max(1)
}

/// Our logind session object. `XDG_SESSION_ID` identifies the session even
/// when the shell runs in a container; otherwise logind resolves "auto"
/// from the caller.
fn session_path() -> String {
    match std::env::var("XDG_SESSION_ID") {
        Ok(id) if !id.is_empty() => format!("/org/freedesktop/login1/session/{}", bus_path_escape(&id)),
        _ => "/org/freedesktop/login1/session/auto".to_owned(),
    }
}

/// sd-bus object-path label escaping: non-alphanumerics and a leading digit
/// become `_xx` (lowercase hex).
fn bus_path_escape(label: &str) -> String {
    let mut out = String::new();
    for (i, b) in label.bytes().enumerate() {
        if b.is_ascii_alphabetic() || (b.is_ascii_digit() && i > 0) {
            out.push(b as char);
        } else {
            out.push_str(&format!("_{b:02x}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_conversion() {
        assert_eq!(to_percent(7500, 7500), 100);
        assert_eq!(to_percent(3750, 7500), 50);
        assert_eq!(to_percent(9999, 7500), 100);
        assert_eq!(from_percent(50, 7500), 3750);
        assert_eq!(from_percent(0, 7500), 1);
        assert_eq!(from_percent(100, 7500), 7500);
    }

    #[test]
    fn escapes_session_ids_like_sd_bus() {
        assert_eq!(bus_path_escape("2"), "_32");
        assert_eq!(bus_path_escape("c1"), "c1");
        assert_eq!(bus_path_escape("a-b"), "a_2db");
    }
}
