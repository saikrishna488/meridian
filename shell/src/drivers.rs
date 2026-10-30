//! Kernel module inventory and authenticated, session-only load/unload actions.
use meridian_protocol::{DriverAction, DriverInfo, ErrorBody, ErrorCode};
use std::{collections::BTreeMap, path::Path, process::Command, sync::Mutex};
static ACTION: Mutex<()> = Mutex::new(());
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 128 && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn module_name(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_str()?.split(".ko").next()?.replace('-', "_");
    valid_name(&name).then_some(name)
}
fn inventory(root: &Path, loaded: &str, sys: &Path) -> Result<Vec<DriverInfo>, ErrorBody> {
    let mut items = BTreeMap::new();
    let mut add = |name: String, path: String, builtin: bool| {
        items.entry(name.clone()).or_insert(DriverInfo {
            name,
            path,
            loaded: builtin,
            builtin,
            users: 0,
            devices: vec![],
        });
    };
    for (file, builtin) in [("modules.dep", false), ("modules.builtin", true)] {
        if let Ok(content) = std::fs::read_to_string(root.join(file)) {
            for line in content.lines() {
                let path = line.split(':').next().unwrap_or("");
                if let Some(name) = module_name(path) {
                    add(name, path.into(), builtin);
                }
            }
        }
    }
    for line in loaded.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 3 || !valid_name(fields[0]) {
            continue;
        }
        let item = items.entry(fields[0].into()).or_insert(DriverInfo {
            name: fields[0].into(),
            path: String::new(),
            loaded: true,
            builtin: false,
            users: 0,
            devices: vec![],
        });
        item.loaded = true;
        item.users = fields[2].parse().unwrap_or(u32::MAX);
    }
    for bus in ["pci", "usb"] {
        if let Ok(devices) = std::fs::read_dir(sys.join("bus").join(bus).join("devices")) {
            for device in devices.flatten() {
                let Ok(module) = std::fs::read_link(device.path().join("driver/module")) else { continue };
                let Some(name) = module.file_name().and_then(|n| n.to_str()) else { continue };
                if let Some(item) = items.get_mut(name) {
                    let product = std::fs::read_to_string(device.path().join("product")).unwrap_or_default();
                    item.devices.push(
                        format!("{} {} {}", bus.to_uppercase(), device.file_name().to_string_lossy(), product.trim())
                            .trim()
                            .into(),
                    );
                }
            }
        }
    }
    Ok(items.into_values().collect())
}
pub fn list() -> Result<Vec<DriverInfo>, ErrorBody> {
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").map_err(error)?;
    let loaded = std::fs::read_to_string("/proc/modules").map_err(error)?;
    inventory(&Path::new("/lib/modules").join(kernel.trim()), &loaded, Path::new("/sys"))
}
pub fn change(choice: DriverAction) -> Result<(), ErrorBody> {
    let _guard = ACTION.try_lock().map_err(|_| error("Another driver change is in progress."))?;
    if !valid_name(&choice.name) {
        return Err(error("Invalid kernel module name."));
    }
    let driver = list()?
        .into_iter()
        .find(|d| d.name == choice.name)
        .ok_or_else(|| error("Driver is no longer available. Refresh the list."))?;
    if driver.builtin {
        return Err(error("This driver is built into the kernel and cannot be unloaded."));
    }
    if driver.loaded == choice.enabled {
        return Ok(());
    }
    if !choice.enabled && driver.users > 0 {
        return Err(error("Driver is in use. Close dependent applications or disconnect its devices first."));
    }
    let modprobe = ["/usr/sbin/modprobe", "/sbin/modprobe", "/usr/bin/modprobe"]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .ok_or_else(|| error("Install the kmod package to manage drivers."))?;
    let mut command = Command::new("/usr/bin/pkexec");
    command.arg(modprobe);
    if !choice.enabled {
        command.arg("--remove");
    }
    let output = command.arg("--").arg(&choice.name).env_remove("MODPROBE_OPTIONS").output().map_err(error)?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(error(if detail.trim().is_empty() {
            "Driver change was cancelled or authentication failed."
        } else {
            detail.trim()
        }));
    }
    let current = list()?.into_iter().find(|d| d.name == choice.name);
    if current.is_none_or(|d| d.loaded != choice.enabled) {
        return Err(error(
            "The driver state did not change. It may have been automatically reloaded or blocked by system policy.",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_options_paths_and_parameters() {
        for name in ["", "--all", "../foo", "foo bar", "foo=1", "foo;id", "a\n"] {
            assert!(!valid_name(name));
        }
        assert!(valid_name("snd_hda_intel"));
        assert_eq!(module_name("kernel/net/test-driver.ko.xz"), Some("test_driver".into()));
    }
    #[test]
    fn merges_loaded_available_and_builtin_modules() {
        let root = std::env::temp_dir().join(format!("meridian-drivers-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("modules.dep"), "kernel/a-driver.ko.xz:\nkernel/unused.ko: kernel/a-driver.ko.xz\n")
            .unwrap();
        std::fs::write(root.join("modules.builtin"), "kernel/builtin.ko\n").unwrap();
        let items = inventory(&root, "a_driver 123 2 - Live 0\nexternal 10 0 - Live 0", &root).unwrap();
        assert_eq!(items.len(), 4);
        assert!(items.iter().any(|d| d.name == "a_driver" && d.loaded && d.users == 2));
        assert!(items.iter().any(|d| d.name == "builtin" && d.builtin && d.loaded));
        assert!(items.iter().any(|d| d.name == "unused" && !d.loaded));
        std::fs::remove_dir_all(root).unwrap();
    }
}
