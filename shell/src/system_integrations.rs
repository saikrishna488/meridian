//! Nonprivileged device discovery and policy-checked time administration.
use meridian_protocol::{ErrorBody, ErrorCode, PrinterInventory, TimeChange, TimeState};
use std::{path::Path, time::Duration};

fn call(args: &[&str]) -> Result<String, ErrorBody> {
    crate::settings_devices::run_timeout(args, None, Duration::from_secs(12))
}
pub fn time() -> Result<TimeState, ErrorBody> {
    let output = call(&[
        "timedatectl", "show", "--property=Timezone", "--property=NTP",
        "--property=NTPSynchronized",
    ])?;
    let find = |name: &str| output.lines().find_map(|line| line.strip_prefix(name)).unwrap_or("").trim();
    Ok(TimeState {
        timezone: find("Timezone=").to_owned(),
        ntp: find("NTP=") == "yes",
        synchronized: find("NTPSynchronized=") == "yes",
    })
}
fn valid_zone(value: &str) -> bool {
    if value.is_empty() || value.len() > 100
        || value.starts_with('/') || value.contains("..") || value.contains("//")
        || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-+/".contains(&b)) { return false; }
    let root = Path::new("/usr/share/zoneinfo");
    let Ok(found) = root.join(value).canonicalize() else { return false };
    found.starts_with(root) && found.is_file()
}
pub fn set_time(change: TimeChange) -> Result<TimeState, ErrorBody> {
    if change.timezone.is_none() && change.ntp.is_none() {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Select a time preference to change"));
    }
    if let Some(zone) = change.timezone {
        if !valid_zone(&zone) {
            return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Unknown or invalid time zone"));
        }
        call(&["timedatectl", "set-timezone", &zone])?;
    }
    if let Some(ntp) = change.ntp {
        call(&["timedatectl", "set-ntp", if ntp { "true" } else { "false" }])?;
    }
    time()
}
pub fn printers() -> Result<PrinterInventory, ErrorBody> {
    let printers = call(&["lpstat", "-p"]);
    let scanners = call(&["scanimage", "-L"]);
    Ok(PrinterInventory {
        cups_available: printers.is_ok(),
        sane_available: scanners.is_ok(),
        printers: printers.unwrap_or_default().lines().filter_map(|line| {
            line.strip_prefix("printer ").and_then(|rest| rest.split_whitespace().next()).map(str::to_owned)
        }).take(128).collect(),
        scanners: scanners.unwrap_or_default().lines().filter(|line| line.starts_with("device "))
            .take(128).map(str::to_owned).collect(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_time_zone_traversal() {
        assert!(!valid_zone("../etc/passwd"));
        assert!(!valid_zone("foo//bar"));
        assert!(!valid_zone("/etc/passwd"));
    }
}
