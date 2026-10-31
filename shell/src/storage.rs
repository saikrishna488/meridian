//! Page-scoped block inventory. lsblk describes topology; UDisks owns authorization
//! and filesystem-specific operations. Never sum disks and their partitions.
use meridian_protocol::{ErrorBody, ErrorCode, StorageAction, StorageState, StorageVolume};
use serde_json::Value;
use std::{collections::HashSet, time::Duration};
static OPERATION: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub(crate) fn operation_guard() -> Result<std::sync::MutexGuard<'static, ()>, ErrorBody> {
    OPERATION.try_lock().map_err(|_| error("Another storage operation is in progress. Wait, refresh, and retry."))
}
fn error(message: impl ToString) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, message.to_string())
}
fn run(args: &[&str]) -> Result<String, ErrorBody> {
    crate::settings_devices::run_timeout(args, None, Duration::from_secs(15))
}
fn tree() -> Result<Value, ErrorBody> {
    let data: Value = serde_json::from_str(&run(&[
        "lsblk",
        "--json",
        "--bytes",
        "--paths",
        "--output",
        "NAME,TYPE,SIZE,FSTYPE,LABEL,MOUNTPOINTS,RO,RM,TRAN",
    ])?)
    .map_err(error)?;
    if !data["blockdevices"].is_array() {
        return Err(error("Invalid block inventory. Refresh Storage."));
    }
    Ok(data)
}
fn mounts(node: &Value) -> Vec<String> {
    node["mountpoints"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_owned)).collect()
}
fn flag(value: &Value) -> bool {
    value.as_bool().unwrap_or_else(|| value.as_u64() == Some(1))
}
// diskseq changes on unplug/replug even if Linux reuses /dev/sdb. Partitions
// inherit their parent disk's sequence. Older kernels can report no sequence.
pub(crate) fn disk_identity(path: &str) -> Option<String> {
    let name = std::path::Path::new(path).file_name()?;
    let device = std::fs::canonicalize(std::path::Path::new("/sys/class/block").join(name)).ok()?;
    for root in [Some(device.as_path()), device.parent()].into_iter().flatten() {
        if let Ok(seq) = std::fs::read_to_string(root.join("diskseq"))
            && let Ok(seq) = seq.trim().parse::<u64>()
        {
            return Some(format!("diskseq:{seq}"));
        }
    }
    None
}
pub(crate) fn validate_identity(expected: Option<&str>, current: Option<&str>) -> Result<(), ErrorBody> {
    if expected.is_some() && expected != current {
        return Err(error("This volume was removed or replaced. Refresh Storage before trying again."));
    }
    Ok(())
}
// Only physical/virtual block storage and its actual volumes belong in Storage.
// Input peripherals, device nodes and non-storage kernel devices do not.
fn storage_kind(kind: &str) -> bool {
    matches!(
        kind,
        "disk" | "part" | "rom" | "crypt" | "lvm" | "raid0" | "raid1"
            | "raid4" | "raid5" | "raid6" | "raid10" | "md" | "mpath"
    )
}
fn inventory(tree: &Value) -> Vec<StorageVolume> {
    fn visit(
        node: &Value,
        parent: Option<&str>,
        removable: bool,
        seen: &mut HashSet<String>,
        out: &mut Vec<StorageVolume>,
    ) {
        let Some(path) = node["name"].as_str().filter(|p| p.starts_with("/dev/")) else {
            return;
        };
        if !seen.insert(path.into()) {
            return;
        }
        let removable = removable || flag(&node["rm"]) || node["tran"] == "usb";
        let kind = node["type"].as_str().unwrap_or("unknown");
        if !storage_kind(kind) {
            // Still visit descendants: a mapper or aggregate can contain a
            // genuine filesystem even if the parent type is not supported.
            for child in node["children"].as_array().into_iter().flatten() {
                visit(child, parent, removable, seen, out);
            }
            return;
        }
        let points = mounts(node);
        let fs = node["fstype"].as_str().unwrap_or_default();
        let mut actions = vec![];
        if points.iter().any(|p| p.starts_with('/')) {
            actions.push("open".into());
        }
        // Encrypted containers, swap, and unknown formats require their own UI.
        let mountable = !fs.is_empty() && !["swap", "crypto_LUKS", "LVM2_member", "linux_raid_member"].contains(&fs);
        if mountable && kind != "loop" && node["mountpoints"].is_array() && crate::device_manager::safe_mounts(node) {
            if points.is_empty() {
                actions.push("mount".into());
            } else {
                actions.push("unmount".into());
            }
        }
        if kind == "disk"
            && node["tran"] == "usb"
            && node["mountpoints"].is_array()
            && crate::device_manager::safe_mounts(node)
        {
            actions.push("eject".into());
        }
        out.push(StorageVolume {
            identity: disk_identity(path),
            path: path.into(),
            parent: parent.map(str::to_owned),
            name: node["label"].as_str().filter(|s| !s.is_empty()).unwrap_or(path).into(),
            kind: kind.into(),
            filesystem: fs.into(),
            size: node["size"].as_f64().unwrap_or(0.),
            mount_points: points,
            read_only: flag(&node["ro"]),
            removable,
            actions,
        });
        for child in node["children"].as_array().into_iter().flatten() {
            visit(child, Some(path), removable, seen, out);
        }
    }
    let mut out = vec![];
    let mut seen = HashSet::new();
    for node in tree["blockdevices"].as_array().into_iter().flatten() {
        visit(node, None, false, &mut seen, &mut out);
    }
    out
}
fn mounted_filesystems(volumes: &mut Vec<StorageVolume>, mountinfo: &str) {
    fn decode(value: &str) -> String {
        value.replace("\\040", " ").replace("\\011", "\t").replace("\\012", "\n").replace("\\134", "\\")
    }
    for line in mountinfo.lines().take(4096) {
        let Some((left, right)) = line.split_once(" - ") else {
            continue;
        };
        let f: Vec<_> = left.split_whitespace().collect();
        let r: Vec<_> = right.split_whitespace().collect();
        if f.len() < 6 || r.len() < 3 {
            continue;
        }
        let fs = r[0];
        let point = decode(f[4]);
        // The block inventory already contains physical disks and volumes.
        // Only supplement it with real remote/shared filesystems. FUSE
        // portals, GVFS mounts, and kernel pseudo mounts are not disks.
        let remote = matches!(
            fs,
            "nfs" | "nfs4" | "cifs" | "smb3" | "sshfs"
                | "fuse.sshfs" | "9p" | "virtiofs" | "ceph" | "glusterfs"
        );
        if !remote || !point.starts_with('/') || volumes.iter().any(|v| v.mount_points.contains(&point)) {
            continue;
        }
        volumes.push(StorageVolume {
            identity: Some(format!("mount:{}:{}:{}", f[0], r[1], f[4])),
            path: format!("mount:{}", f[0]),
            name: format!("{} · {}", decode(r[1]), point),
            kind: "mounted filesystem".into(),
            filesystem: fs.into(),
            size: 0.,
            mount_points: vec![point],
            read_only: f[5].split(',').any(|o| o == "ro"),
            removable: false,
            actions: vec!["open".into()],
            parent: None,
        });
    }
}
fn append_mounts(volumes: &mut Vec<StorageVolume>) {
    use std::io::Read;
    let mut data = String::new();
    if let Ok(file) = std::fs::File::open("/proc/self/mountinfo")
        && file.take(1024 * 1024).read_to_string(&mut data).is_ok()
    {
        mounted_filesystems(volumes, &data);
    }
}
pub fn list() -> Result<StorageState, ErrorBody> {
    let (mut volumes, discovery_warning) = match tree() {
        Ok(tree) => (inventory(&tree), None),
        Err(e) => (vec![], Some(format!("Disk discovery: {}", e.message))),
    };
    append_mounts(&mut volumes);
    let mut state = StorageState {
        volumes,
        total: 0.,
        available: 0.,
        filesystem: String::new(),
        source: String::new(),
        warning: discovery_warning,
    };
    // df reports the mounted root filesystem, including network/overlay/Btrfs,
    // without adding subvolumes or bind mounts to a misleading disk total.
    match run(&["df", "--block-size=1", "--output=source,fstype,size,avail", "/"]) {
        Ok(output) => {
            if let Some(line) = output.lines().nth(1) {
                let f: Vec<_> = line.split_whitespace().collect();
                if f.len() == 4 {
                    state.source = f[0].into();
                    state.filesystem = f[1].into();
                    state.total = f[2].parse().unwrap_or(0.);
                    state.available = f[3].parse().unwrap_or(0.);
                }
            }
            if state.total <= 0. {
                state.warning = Some("Root filesystem capacity is unavailable.".into());
            }
        }
        Err(e) => state.warning = Some(format!("{}{}", state.warning.unwrap_or_default(), e.message)),
    }
    Ok(state)
}
fn resolve(choice: &StorageAction) -> Result<StorageVolume, ErrorBody> {
    // Revalidate against fresh topology immediately before touching a volume.
    let mut volumes = match tree() {
        Ok(data) => inventory(&data),
        Err(_) if choice.action == "open" => vec![],
        Err(e) => return Err(e),
    };
    append_mounts(&mut volumes);
    let volume = volumes
        .into_iter()
        .find(|v| v.path == choice.id)
        .ok_or_else(|| error("Volume disconnected. Refresh Storage."))?;
    validate_identity(choice.identity.as_deref(), volume.identity.as_deref())?;
    if !volume.actions.contains(&choice.action) {
        return Err(error("This operation is unavailable or protects active system storage."));
    }
    Ok(volume)
}
pub async fn perform(choice: StorageAction) -> Result<(), ErrorBody> {
    if choice.action == "open" {
        let uri = crate::files::blocking(move || {
            let volume = resolve(&choice)?;
            let path = volume
                .mount_points
                .iter()
                .find(|p| p.starts_with('/'))
                .ok_or_else(|| error("Volume is not mounted"))?;
            use gtk::gio::prelude::FileExt;
            Ok(gtk::gio::File::for_path(path).uri().to_string())
        })
        .await?;
        gtk::glib::future_with_timeout(
            Duration::from_secs(10),
            gtk::gio::AppInfo::launch_default_for_uri_future(&uri, None::<&gtk::gio::AppLaunchContext>),
        )
        .await
        .map_err(|_| error("File manager launch timed out. Check the default file manager and retry."))?
        .map_err(error)?;
        return Ok(());
    }
    crate::files::blocking(move || action(choice)).await
}
fn action(choice: StorageAction) -> Result<(), ErrorBody> {
    let _guard = operation_guard()?;
    let volume = resolve(&choice)?;
    match choice.action.as_str() {
        "mount" | "unmount" => {
            run(&["udisksctl", &choice.action, "--block-device", &volume.path])?;
        }
        "eject" => {
            crate::device_manager::eject_disk_checked(
                volume.path.strip_prefix("/dev/").ok_or_else(|| error("Invalid volume"))?,
                volume.identity.as_deref(),
            )?;
        }
        _ => return Err(error("Unsupported storage operation")),
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_real_block_storage_and_remote_shares_appear() {
        let data = serde_json::json!({"blockdevices":[
            {"name":"/dev/sda","type":"disk","size":1000,
             "children":[{"name":"/dev/sda1","type":"part","fstype":"ext4","mountpoints":["/"]}]},
            {"name":"/dev/input/event0","type":"input","size":0},
            {"name":"/dev/hidraw0","type":"hid","size":0},
            {"name":"/dev/loop0","type":"loop","size":100}
        ]});
        let inventory = inventory(&data);
        assert_eq!(inventory.len(), 2);
        assert!(inventory.iter().all(|v| v.path.starts_with("/dev/sda")));
        let mut mounts = vec![];
        mounted_filesystems(&mut mounts,
            "1 0 0:1 / /run/user/1000/gvfs rw - fuse.gvfsd-fuse gvfs rw\n2 0 0:2 / /mnt/nas rw - nfs4 server:/share rw");
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].filesystem, "nfs4");
    }
    #[test]
    fn reused_device_paths_reject_a_stale_attachment_identity() {
        assert!(validate_identity(Some("diskseq:10"), Some("diskseq:11")).is_err());
        assert!(validate_identity(Some("diskseq:10"), None).is_err());
        assert!(validate_identity(Some("diskseq:10"), Some("diskseq:10")).is_ok());
        assert!(validate_identity(None, Some("diskseq:10")).is_ok()); // legacy clients
    }
    #[test]
    fn concurrent_storage_mutations_fail_without_waiting() {
        let _guard = operation_guard().unwrap();
        assert!(operation_guard().is_err());
    }
    #[test]
    fn network_and_virtual_mounts_do_not_offer_block_operations() {
        let mut volumes = vec![];
        mounted_filesystems(
            &mut volumes,
            "10 1 0:3 / /run rw - tmpfs tmpfs rw\n11 1 0:4 / /mnt/Shared\\040Files ro - virtiofs shared ro\n12 1 0:5 / /mnt/nas rw - nfs4 server:/share rw",
        );
        assert_eq!(volumes.len(), 2);
        assert_eq!(volumes[0].mount_points, ["/mnt/Shared Files"]);
        assert!(volumes[0].read_only);
        assert!(volumes.iter().all(|v| v.actions == ["open"] && v.size == 0.));
    }
    #[test]
    fn deduplicates_mapper_devices_and_protects_system_and_encrypted_volumes() {
        let data = serde_json::json!({"blockdevices":[
            {"name":"/dev/sda","type":"disk","tran":"usb","children":[{"name":"/dev/sda1","type":"part","fstype":"ext4","mountpoints":["/"]}]},
            {"name":"/dev/sdb1","type":"part","fstype":"exfat","mountpoints":["/run/media/user/USB"]},
            {"name":"/dev/dm-0","type":"crypt","fstype":"ext4","mountpoints":["/home"]},
            {"name":"/dev/dm-0","type":"crypt"},
            {"name":"/dev/sdc1","type":"part","fstype":"crypto_LUKS","mountpoints":[null]}
        ]});
        let v = inventory(&data);
        assert_eq!(v.len(), 5);
        assert!(v[0].actions.is_empty());
        assert_eq!(v[1].actions, ["open"]);
        assert_eq!(v[2].actions, ["open", "unmount"]);
        assert_eq!(v[3].actions, ["open"]);
        assert!(v[4].actions.is_empty());
    }
}
