//! Nova only accepts package names from its bundled catalog.
use meridian_protocol::{DeviceAction, ErrorBody, ErrorCode};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub struct Entry {
    id: String,
    name: String,
    category: String,
    description: String,
    #[serde(default)]
    installed: bool,
    #[serde(default)]
    available: bool,
}
fn catalog() -> Vec<Entry> {
    serde_json::from_str(include_str!("../../ui/src/surfaces/utilities/catalog.json")).expect("bundled catalog")
}
fn error(s: impl ToString) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, s.to_string())
}
pub fn list() -> Result<Vec<Entry>, ErrorBody> {
    let mut entries = catalog();
    let installed = crate::settings_devices::run_timeout(
        &["dpkg-query", "-W", "-f=${Package} ${db:Status-Status}\\n"],
        None,
        std::time::Duration::from_secs(15),
    )?;
    let names: Vec<_> = entries.iter().map(|e| e.id.as_str()).collect();
    let mut args = vec!["apt-cache", "policy"];
    args.extend(names);
    let policy = crate::settings_devices::run_timeout(&args, None, std::time::Duration::from_secs(15))?;
    for entry in &mut entries {
        entry.installed = installed.lines().any(|l| l == format!("{} installed", entry.id));
        entry.available = policy
            .split(&format!("{}:\n", entry.id))
            .nth(1)
            .and_then(|s| s.lines().find(|l| l.trim().starts_with("Candidate:")))
            .is_some_and(|l| !l.contains("(none)"));
    }
    Ok(entries)
}
thread_local! { static BUSY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        BUSY.set(false);
    }
}
pub async fn action(choice: DeviceAction) -> Result<(), ErrorBody> {
    if !catalog().iter().any(|e| e.id == choice.id) {
        return Err(error("Unknown catalog app"));
    }
    if !["install", "uninstall"].contains(&choice.action.as_str()) {
        return Err(error("Unsupported store action"));
    }
    if BUSY.replace(true) {
        return Err(error("Another Nova package operation is running"));
    }
    let _guard = Guard;
    if choice.action == "install" {
        crate::uninstall::run(&["pkexec", "apt-get", "--assume-yes", "--no-remove", "install", "--", &choice.id])
            .await?;
    } else {
        // Refuse removal when it would remove any other package.
        let plan = crate::uninstall::run(&["apt-get", "--simulate", "remove", "--", &choice.id]).await?;
        if plan
            .lines()
            .filter_map(|l| l.strip_prefix("Remv "))
            .any(|l| l.split_whitespace().next() != Some(choice.id.as_str()))
        {
            return Err(error(
                "Removing this app would remove other packages. Use a system package manager to review that change.",
            ));
        }
        crate::uninstall::run(&["pkexec", "apt-get", "--assume-yes", "remove", "--", &choice.id]).await?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_100_unique_safe_packages() {
        let entries = catalog();
        assert_eq!(entries.len(), 100);
        let mut ids = std::collections::HashSet::new();
        for e in entries {
            assert!(ids.insert(e.id.clone()));
            assert!(e.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+-".contains(&b)));
        }
    }
}
