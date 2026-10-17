//! Browse GIO's trash namespace and restore without overwriting destinations.
use gtk::{gio, prelude::*};
use meridian_protocol::{ErrorBody, ErrorCode, TrashEntry, TrashListing};
use std::path::Path;
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
pub fn list() -> Result<TrashListing, ErrorBody> {
    let root = gio::File::for_uri("trash:///");
    let enumerator = root
        .enumerate_children(
            "standard::name,standard::display-name,standard::type,standard::size,trash::orig-path,trash::deletion-date",
            gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
            gio::Cancellable::NONE,
        )
        .map_err(error)?;
    let mut entries = vec![];
    let mut truncated = false;
    while let Some(info) = enumerator.next_file(gio::Cancellable::NONE).map_err(error)? {
        if entries.len() >= 5000 {
            truncated = true;
            break;
        }
        entries.push(TrashEntry {
            uri: enumerator.child(&info).uri().to_string(),
            name: info.display_name().to_string(),
            original: info
                .attribute_file_path("trash::orig-path")
                .and_then(|p| p.to_str().map(str::to_owned))
                .unwrap_or_default(),
            deleted: info.attribute_string("trash::deletion-date").map(|s| s.to_string()).unwrap_or_default(),
            size: info.size().max(0) as f64,
            directory: info.file_type() == gio::FileType::Directory,
        });
    }
    let (_, close_error) = enumerator.close(gio::Cancellable::NONE);
    if let Some(e) = close_error {
        return Err(error(e));
    }
    entries.sort_by(|a, b| b.deleted.cmp(&a.deleted).then(a.name.cmp(&b.name)));
    Ok(TrashListing { entries, truncated })
}
fn destination(original: &str) -> Result<&Path, ErrorBody> {
    let path = Path::new(original);
    if !path.is_absolute()
        || path.file_name().is_none()
        || path.components().any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(error("The original location of this item is unavailable."));
    }
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(error("The original folder no longer exists. Create it before restoring this item."));
    }
    Ok(path)
}
pub fn restore(uri: &str) -> Result<String, ErrorBody> {
    // Only accept exact item URIs returned by our top-level trash listing.
    let entry = list()?
        .entries
        .into_iter()
        .find(|entry| entry.uri == uri)
        .ok_or_else(|| error("This item is no longer in Trash. Refresh and try again."))?;
    let target = destination(&entry.original)?;
    restore_to(&gio::File::for_uri(uri), target)?;
    Ok(entry.original)
}
fn restore_to(source: &gio::File, target: &Path) -> Result<(), ErrorBody> {
    source.move_(&gio::File::for_path(target), gio::FileCopyFlags::NOFOLLOW_SYMLINKS, gio::Cancellable::NONE, None)
        .map_err(|e| if e.matches(gio::IOErrorEnum::Exists) {
            error("An item with the same name already exists in the original folder. Rename or move it before restoring.")
        } else { error(e) })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a running session bus and GVfs trash service"]
    fn round_trip_through_real_gvfs_trash_uses_only_its_own_fixture() {
        let root = gtk::glib::user_cache_dir().join(format!(
            "meridian-trash-roundtrip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("fixture.txt");
        std::fs::write(&path, "Meridian restore fixture").unwrap();
        gio::File::for_path(&path).trash(gio::Cancellable::NONE).unwrap();
        let entry = list()
            .unwrap()
            .entries
            .into_iter()
            .find(|entry| entry.original == path.to_str().unwrap())
            .expect("fixture should be listed in Trash");
        assert!(!path.exists());
        assert_eq!(restore(&entry.uri).unwrap(), path.to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Meridian restore fixture");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn restore_never_overwrites_files_and_rejects_invalid_original_locations() {
        let root = std::env::temp_dir().join(format!("meridian-restore-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("trashed.txt");
        let target = root.join("original.txt");
        std::fs::write(&source, "restore me").unwrap();
        std::fs::write(&target, "keep me").unwrap();
        let file = gio::File::for_path(&source);
        assert!(restore_to(&file, &target).is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep me");
        assert!(source.exists());
        std::fs::remove_file(&target).unwrap();
        restore_to(&file, &target).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "restore me");
        for path in ["relative", "/", "/tmp/../etc/file", ""] {
            assert!(destination(path).is_err());
        }
        assert!(destination(root.join("missing/file").to_str().unwrap()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
