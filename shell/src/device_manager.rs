//! Hardware-oriented inventory. Actions are resolved against a fresh inventory.
use meridian_protocol::{DeviceAction, ErrorBody, ErrorCode, ManagedDevice};
use std::{fs, path::Path, time::Duration};
fn text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default().trim().into()
}
fn error(s: impl ToString) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, s.to_string())
}
fn run(args: &[&str]) -> Result<String, ErrorBody> {
    crate::settings_devices::run_timeout(args, None, Duration::from_secs(12))
}
fn entries(path: &Path) -> Vec<(String, std::path::PathBuf)> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect()
}
fn driver(path: &Path) -> String {
    let root = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    for parent in root.ancestors().take(8) {
        for suffix in ["driver/module", "driver", "device/driver/module", "device/driver"] {
            if let Ok(target) = fs::read_link(parent.join(suffix))
                && let Some(name) = target.file_name()
            {
                return name.to_string_lossy().into_owned();
            }
        }
    }
    "Unavailable".into()
}
fn item(id: String, category: &str, name: String, status: String, path: &Path) -> ManagedDevice {
    ManagedDevice {
        id,
        category: category.into(),
        name,
        status,
        properties: vec![("Driver".into(), driver(path))],
        actions: vec![],
    }
}
fn sys_devices(sys: &Path) -> Vec<ManagedDevice> {
    let mut out = vec![];
    for (name, p) in entries(&sys.join("class/net")) {
        if name == "lo" {
            continue;
        }
        let wireless = p.join("wireless").exists() || p.join("phy80211").exists();
        let mut d = item(
            format!("net:{name}"),
            if wireless { "Wi-Fi adapters" } else { "Network adapters" },
            name.clone(),
            text(p.join("operstate")),
            &p,
        );
        d.properties.extend([("Interface".into(), name), ("Address".into(), text(p.join("address")))]);
        out.push(d);
    }
    for (name, p) in entries(&sys.join("class/bluetooth")) {
        if !name.strip_prefix("hci").is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())) {
            continue;
        }
        let mut d = item(format!("bluetooth:{name}"), "Bluetooth", name.clone(), "Unavailable".into(), &p);
        d.properties.push(("Adapter".into(), name));
        out.push(d);
    }
    for (name, p) in entries(&sys.join("block")) {
        if ["loop", "ram", "zram"].iter().any(|s| name.starts_with(s)) {
            continue;
        }
        let model = text(p.join("device/model"));
        let mut d = item(
            format!("storage:{name}"),
            "Storage",
            if model.is_empty() { name.clone() } else { model },
            "Connected".into(),
            &p,
        );
        d.properties.push(("Device".into(), format!("/dev/{name}")));
        if let Ok(sectors) = text(p.join("size")).parse::<u64>() {
            d.properties.push(("Capacity".into(), format!("{:.1} GB", sectors as f64 * 512.0 / 1e9)));
        }
        out.push(d);
    }
    for bus in ["pci", "usb"] {
        for (name, p) in entries(&sys.join("bus").join(bus).join("devices")) {
            // USB interfaces duplicate the physical device.
            if bus == "usb" && name.contains(':') {
                continue;
            }
            let product = text(p.join("product"));
            let label = if product.is_empty() { format!("{} {}", bus.to_uppercase(), name) } else { product };
            let mut d = item(
                format!("{bus}:{name}"),
                if bus == "usb" { "USB devices" } else { "System devices" },
                label,
                "Connected".into(),
                &p,
            );
            d.properties.push(("Location".into(), name));
            let vendor = text(p.join(if bus == "usb" { "idVendor" } else { "vendor" }));
            let product = text(p.join(if bus == "usb" { "idProduct" } else { "device" }));
            d.properties.push(("Hardware ID".into(), format!("{vendor}:{product}")));
            out.push(d);
        }
    }
    out
}
// lsblk includes nested partitions, mapper devices, mounts and active swap.
fn safe_mounts(node: &serde_json::Value) -> bool {
    let mounts_safe = node["mountpoints"].as_array().is_none_or(|mounts| {
        mounts.iter().all(|m| {
            m.as_str().is_none_or(|p| {
                if p.starts_with("/run/media/") {
                    return true;
                }
                p.starts_with('/')
                    && p != "/"
                    && ![
                        "/boot", "/home", "/usr", "/var", "/etc", "/opt", "/bin", "/sbin", "/lib", "/lib64", "/run",
                        "/tmp", "/dev", "/proc", "/sys", "/root",
                    ]
                    .iter()
                    .any(|prefix| p == *prefix || p.starts_with(&format!("{prefix}/")))
            })
        })
    });
    mounts_safe && node["children"].as_array().is_none_or(|items| items.iter().all(safe_mounts))
}
fn storage_tree() -> Result<serde_json::Value, ErrorBody> {
    serde_json::from_str(&run(&["lsblk", "--json", "--paths", "--output", "NAME,TYPE,MOUNTPOINTS,TRAN"])?)
        .map_err(error)
}
fn storage_actions(devices: &mut Vec<ManagedDevice>, tree: &serde_json::Value) {
    fn visit(node: &serde_json::Value, devices: &mut Vec<ManagedDevice>) {
        let path = node["name"].as_str().unwrap_or_default();
        let name = path.strip_prefix("/dev/").unwrap_or_default();
        if !name.is_empty() {
            let id = format!("storage:{name}");
            if !devices.iter().any(|d| d.id == id) && node["type"] == "part" {
                devices.push(item(
                    id.clone(),
                    "Storage",
                    name.into(),
                    "Connected".into(),
                    &Path::new("/sys/class/block").join(name),
                ));
            }
            if let Some(d) = devices.iter_mut().find(|d| d.id == id) {
                if safe_mounts(node) {
                    if node["type"] == "disk" && node["tran"] == "usb" {
                        d.actions.push("eject".into());
                    }
                    if node["type"] == "part"
                        && node["mountpoints"].as_array().is_some_and(|m| m.iter().any(|m| m.is_string()))
                    {
                        d.actions.push("unmount".into());
                    }
                }
                d.properties.push((
                    "Mount points".into(),
                    node["mountpoints"]
                        .as_array()
                        .map(|m| m.iter().filter_map(|m| m.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default(),
                ));
            }
        }
        if let Some(children) = node["children"].as_array() {
            for child in children {
                visit(child, devices);
            }
        }
    }
    if let Some(nodes) = tree["blockdevices"].as_array() {
        for node in nodes {
            visit(node, devices);
        }
    }
    // USB storage devices get the same safe action as their disk entry.
    let disks: Vec<_> = devices
        .iter()
        .filter(|d| d.actions.iter().any(|a| a == "eject"))
        .filter_map(|d| d.id.strip_prefix("storage:").map(str::to_owned))
        .collect();
    for d in devices.iter_mut().filter(|d| d.id.starts_with("usb:")) {
        let usb_path = fs::canonicalize(Path::new("/sys/bus/usb/devices").join(&d.id[4..])).ok();
        for disk in &disks {
            if let (Some(usb), Ok(block)) = (&usb_path, fs::canonicalize(Path::new("/sys/class/block").join(disk)))
                && block.starts_with(usb)
            {
                d.actions.push(format!("eject:{disk}"));
            }
        }
    }
}
fn unmount_tree(node: &serde_json::Value) -> Result<(), ErrorBody> {
    if !safe_mounts(node) {
        return Err(error("System storage cannot be ejected."));
    }
    if let Some(children) = node["children"].as_array() {
        for child in children {
            unmount_tree(child)?;
        }
    }
    if node["mountpoints"].as_array().is_some_and(|m| m.iter().any(|m| m.is_string())) {
        let path = node["name"].as_str().ok_or_else(|| error("Missing block device"))?;
        run(&["udisksctl", "unmount", "--block-device", path])?;
    }
    Ok(())
}
fn eject_disk(name: &str) -> Result<(), ErrorBody> {
    let tree = storage_tree()?;
    let path = format!("/dev/{name}");
    let node = tree["blockdevices"]
        .as_array()
        .and_then(|items| items.iter().find(|n| n["name"] == path && n["type"] == "disk" && n["tran"] == "usb"))
        .ok_or_else(|| error("USB disk is no longer available"))?;
    unmount_tree(node)?;
    run(&["udisksctl", "power-off", "--block-device", &path])?;
    Ok(())
}
pub fn list() -> Result<Vec<ManagedDevice>, ErrorBody> {
    let mut devices = sys_devices(Path::new("/sys"));
    for d in &mut devices {
        if let Some(name) = d.id.strip_prefix("net:")
            && let Ok(state) = run(&["nmcli", "-g", "GENERAL.STATE", "device", "show", name])
        {
            let connected = state.trim().starts_with("100");
            d.status = state
                .trim()
                .split_once('(')
                .map(|(_, s)| s.trim_end_matches(')').to_owned())
                .unwrap_or_else(|| state.trim().into());
            if !state.trim().starts_with("10 ") && !state.trim().starts_with("20 ") {
                d.actions.push(if connected { "disconnect" } else { "connect" }.into());
            }
        }
        if let Some(name) = d.id.strip_prefix("bluetooth:") {
            let path = format!("/org/bluez/{name}");
            if let Ok(state) =
                run(&["busctl", "--system", "get-property", "org.bluez", &path, "org.bluez.Adapter1", "Powered"])
            {
                let on = state.trim() == "b true";
                d.status = if on { "On" } else { "Off" }.into();
                d.actions.push(if on { "off" } else { "on" }.into());
            }
        }
    }
    if let Ok(tree) = storage_tree() {
        storage_actions(&mut devices, &tree);
    }
    if let Ok(printers) = run(&["lpstat", "-p"]) {
        devices.extend(printer_devices(&printers));
    }
    devices.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.name.cmp(&b.name)));
    Ok(devices)
}
fn printer_devices(output: &str) -> Vec<ManagedDevice> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix("printer ")?;
            let (name, state) = line.split_once(' ')?;
            let paused = state.contains("disabled");
            Some(ManagedDevice {
                id: format!("printer:{name}"),
                category: "Printers".into(),
                name: name.into(),
                status: if paused { "Paused" } else { "Ready" }.into(),
                properties: vec![("Queue".into(), name.into())],
                actions: vec![if paused { "resume" } else { "pause" }.into()],
            })
        })
        .collect()
}
pub fn action(choice: DeviceAction) -> Result<(), ErrorBody> {
    let device = list()?
        .into_iter()
        .find(|d| d.id == choice.id)
        .ok_or_else(|| error("Device is no longer connected. Refresh and try again."))?;
    if !device.actions.contains(&choice.action) {
        return Err(error("This action is unavailable for this device."));
    }
    let (kind, name) = device.id.split_once(':').ok_or_else(|| error("Invalid device"))?;
    if name.is_empty()
        || name.starts_with('-')
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
    {
        return Err(error("Invalid device name"));
    }
    if kind == "usb"
        && let Some(disk) = choice.action.strip_prefix("eject:")
    {
        return eject_disk(disk);
    }
    match (kind, choice.action.as_str()) {
        ("storage", "eject") => eject_disk(name)?,
        ("storage", "unmount") => {
            run(&["udisksctl", "unmount", "--block-device", &format!("/dev/{name}")])?;
        }

        ("net", "connect" | "disconnect") => {
            run(&["nmcli", "--wait", "10", "device", &choice.action, name])?;
        }
        ("bluetooth", "on" | "off") => {
            run(&[
                "busctl",
                "--system",
                "set-property",
                "org.bluez",
                &format!("/org/bluez/{name}"),
                "org.bluez.Adapter1",
                "Powered",
                "b",
                if choice.action == "on" { "true" } else { "false" },
            ])?;
        }
        ("printer", "pause" | "resume") => {
            let name_of_program = if choice.action == "pause" { "cupsdisable" } else { "cupsenable" };
            let program = ["/usr/sbin", "/usr/bin", "/sbin"]
                .map(|dir| std::path::Path::new(dir).join(name_of_program))
                .into_iter()
                .find(|path| path.is_file())
                .ok_or_else(|| error("Install the CUPS tools to manage printers."))?;
            let program = program.to_str().ok_or_else(|| error("Invalid CUPS program path"))?;
            crate::settings_devices::run_timeout(&["pkexec", program, name], None, Duration::from_secs(60))?;
        }
        _ => return Err(error("Unsupported device action")),
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_network_and_storage_and_reports_the_bound_driver() {
        let root = std::env::temp_dir().join(format!("meridian-hardware-{}", std::process::id()));
        fs::create_dir_all(root.join("class/net/wlan0/wireless")).unwrap();
        fs::create_dir_all(root.join("block/sda/device")).unwrap();
        fs::write(root.join("class/net/wlan0/operstate"), "up").unwrap();
        fs::write(root.join("block/sda/device/model"), "Test SSD").unwrap();
        fs::write(root.join("block/sda/size"), "2097152").unwrap();
        std::os::unix::fs::symlink("/sys/module/sd_mod", root.join("block/sda/device/driver")).unwrap();
        let devices = sys_devices(&root);
        assert!(devices.iter().any(|d| d.category == "Wi-Fi adapters" && d.name == "wlan0"));
        let storage = devices.iter().find(|d| d.category == "Storage").unwrap();
        assert_eq!(storage.name, "Test SSD");
        assert!(storage.actions.is_empty());
        assert!(storage.properties.contains(&("Driver".into(), "sd_mod".into())));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn printer_state_and_actions_match_queue_state() {
        let devices =
            printer_devices("printer Office is idle. enabled since today\nprinter Photo disabled since today\n");
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].actions, ["pause"]);
        assert_eq!(devices[1].actions, ["resume"]);
    }
    #[test]
    fn system_mounts_and_swap_are_protected_recursively() {
        for mount in ["/", "/boot", "/boot/efi", "/home", "/usr", "/var/lib", "[SWAP]"] {
            assert!(!safe_mounts(&serde_json::json!({"children":[{"mountpoints":[mount]}]})));
        }
        assert!(safe_mounts(&serde_json::json!({"children":[{"mountpoints":["/media/user/USB", "/data",null]}]})));
    }
    #[test]
    fn exposes_only_safe_disk_and_partition_actions() {
        let tree = serde_json::json!({"blockdevices":[
            {"name":"/dev/sda","type":"disk","tran":"usb","mountpoints":[null],"children":[
                {"name":"/dev/sda1","type":"part","mountpoints":["/"]},
                {"name":"/dev/sda2","type":"part","mountpoints":["/data"]}]},
            {"name":"/dev/sdb","type":"disk","tran":"usb","mountpoints":[null],"children":[
                {"name":"/dev/sdb1","type":"part","mountpoints":["/media/user/drive"]}]}
        ]});
        let mut devices = vec![
            item("storage:sda".into(), "Storage", "System".into(), "".into(), Path::new("/nonexistent")),
            item("storage:sdb".into(), "Storage", "USB".into(), "".into(), Path::new("/nonexistent")),
        ];
        storage_actions(&mut devices, &tree);
        let actions = |id: &str| devices.iter().find(|d| d.id == id).unwrap().actions.clone();
        assert!(!actions("storage:sda").contains(&"eject".into()));
        assert!(actions("storage:sda1").is_empty());
        assert_eq!(actions("storage:sda2"), ["unmount"]);
        assert_eq!(actions("storage:sdb"), ["eject"]);
        assert_eq!(actions("storage:sdb1"), ["unmount"]);
    }
    #[test]
    fn absent_sysfs_is_an_empty_inventory() {
        assert!(sys_devices(Path::new("/nonexistent/meridian-devices")).is_empty());
    }
}
