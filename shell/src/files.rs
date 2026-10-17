//! Finder operations. Blocking filesystem work stays off the GTK main loop.
//! Renames never overwrite, and removal always uses the system trash.
use gtk::prelude::*;
use gtk::{gio, glib};
use meridian_protocol::{ErrorBody, ErrorCode, FileEntry, FileListing, FileLocation, FileProperties};
use std::path::{Path, PathBuf};

fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
fn absolute(path: &str) -> Result<PathBuf, ErrorBody> {
    let path = PathBuf::from(path);
    if !path.is_absolute() || path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(ErrorBody::new(
            ErrorCode::InvalidRequest,
            "Choose an absolute file path without parent components.",
        ));
    }
    Ok(path)
}
pub fn directory_path(path: &str) -> Result<PathBuf, ErrorBody> {
    let path = absolute(path)?.canonicalize().map_err(error)?;
    if !path.is_dir() {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Choose a folder to open in Terminal."));
    }
    Ok(path)
}
fn valid_name(name: &str) -> Result<(), ErrorBody> {
    if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\0']) {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Enter a name without slashes."));
    }
    Ok(())
}
pub fn locations() -> Vec<FileLocation> {
    let mut result = vec![FileLocation { name: "Home".into(), path: glib::home_dir().to_string_lossy().into_owned() }];
    for (name, directory) in [
        ("Desktop", glib::UserDirectory::Desktop),
        ("Documents", glib::UserDirectory::Documents),
        ("Downloads", glib::UserDirectory::Downloads),
        ("Pictures", glib::UserDirectory::Pictures),
        ("Music", glib::UserDirectory::Music),
        ("Videos", glib::UserDirectory::Videos),
    ] {
        if let Some(path) = glib::user_special_dir(directory).filter(|p| p.is_dir()) {
            result.push(FileLocation { name: name.into(), path: path.to_string_lossy().into_owned() });
        }
    }
    result.push(FileLocation { name: "Computer".into(), path: "/".into() });
    if let Ok(mounts) = std::fs::read_dir(format!("/media/{}", glib::user_name().to_string_lossy())) {
        for entry in mounts.flatten().filter(|e| e.path().is_dir()) {
            result.push(FileLocation {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path().to_string_lossy().into_owned(),
            });
        }
    }
    result
}
pub async fn blocking<T: Send + 'static>(
    action: impl FnOnce() -> Result<T, ErrorBody> + Send + 'static,
) -> Result<T, ErrorBody> {
    gio::spawn_blocking(action).await.map_err(|_| error("File operation failed"))?
}
pub fn list(path: &str, hidden: bool) -> Result<FileListing, ErrorBody> {
    let path = absolute(path)?.canonicalize().map_err(error)?;
    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in std::fs::read_dir(&path).map_err(error)? {
        let entry = entry.map_err(error)?;
        // Non-UTF8 filenames cannot round-trip safely through the JSON bridge.
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !hidden && name.starts_with('.') {
            continue;
        }
        let symlink = entry.file_type().map_err(error)?.is_symlink();
        let meta = std::fs::metadata(entry.path()).or_else(|_| entry.metadata()).map_err(error)?;
        let Some(item_path) = entry.path().to_str().map(str::to_owned) else {
            continue;
        };
        entries.push(FileEntry {
            name,
            path: item_path,
            directory: meta.is_dir(),
            symlink,
            size: meta.len() as f64,
            modified: meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0., |m| m.as_secs_f64()),
        });
        if entries.len() == 5000 {
            truncated = true;
            break;
        }
    }
    entries.sort_by(|a, b| b.directory.cmp(&a.directory).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(FileListing {
        parent: path.parent().map(|p| p.to_string_lossy().into_owned()),
        path: path.to_string_lossy().into_owned(),
        entries,
        truncated,
    })
}
pub fn mkdir(parent: &str, name: &str) -> Result<(), ErrorBody> {
    valid_name(name)?;
    std::fs::create_dir(absolute(parent)?.join(name)).map_err(error)
}

pub fn properties(path: &str) -> Result<FileProperties, ErrorBody> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let path = absolute(path)?;
    // Inspect the link itself, including a broken link, without walking a folder.
    let meta = std::fs::symlink_metadata(&path).map_err(error)?;
    let file = gio::File::for_path(&path);
    let info = file
        .query_info(
            "standard::content-type,owner::user,owner::group",
            gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
            gio::Cancellable::NONE,
        )
        .ok();
    let kind = if meta.is_symlink() {
        "Symbolic link".into()
    } else if meta.is_dir() {
        "Folder".into()
    } else {
        info.as_ref()
            .and_then(|i| i.content_type())
            .map(|t| gio::content_type_get_description(&t).to_string())
            .unwrap_or_else(|| "File".into())
    };
    let time = |t: Result<std::time::SystemTime, std::io::Error>| {
        t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64())
    };
    let attribute = |key: &str, fallback: u32| {
        info.as_ref()
            .and_then(|i| i.attribute_string(key))
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    };
    let mode = meta.permissions().mode();
    let permissions: String = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ]
    .into_iter()
    .map(|(bit, letter)| if mode & bit != 0 { letter } else { '-' })
    .collect();
    Ok(FileProperties {
        name: path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into()),
        path: path.to_string_lossy().into_owned(),
        kind,
        size: if meta.is_file() { Some(meta.len() as f64) } else { None },
        modified: time(meta.modified()),
        created: time(meta.created()),
        owner: attribute("owner::user", meta.uid()),
        group: attribute("owner::group", meta.gid()),
        permissions: format!("{permissions} ({:03o})", mode & 0o7777),
        link_target: if meta.is_symlink() {
            std::fs::read_link(&path).ok().map(|s| s.to_string_lossy().into_owned())
        } else {
            None
        },
    })
}
pub fn rename(path: &str, name: &str) -> Result<(), ErrorBody> {
    valid_name(name)?;
    let source = absolute(path)?;
    let parent = source.parent().ok_or_else(|| error("Cannot rename the filesystem root"))?;
    if source.file_name().is_none() {
        return Err(error("Cannot rename the filesystem root"));
    }
    gio::File::for_path(&source)
        .move_(
            &gio::File::for_path(parent.join(name)),
            gio::FileCopyFlags::NOFOLLOW_SYMLINKS,
            gio::Cancellable::NONE,
            None,
        )
        .map_err(error)
}
pub fn trash(path: &str) -> Result<(), ErrorBody> {
    let path = absolute(path)?;
    // Never offer to trash the filesystem root or a user's home itself.
    let canonical = path.canonicalize().map_err(error)?;
    if canonical == Path::new("/") || canonical == glib::home_dir() {
        return Err(error("This location cannot be moved to Trash."));
    }
    gio::File::for_path(path).trash(gio::Cancellable::NONE).map_err(error)
}
pub fn open(path: &str) -> Result<(), ErrorBody> {
    let path = absolute(path)?;
    if !path.exists() {
        return Err(error("The file no longer exists."));
    }
    let mut argv: Vec<std::ffi::OsString> = Vec::new();
    if std::env::var("MERIDIAN_HOST_APPS").as_deref() == Ok("1") {
        argv.extend(["flatpak-spawn".into(), "--host".into()]);
    }
    argv.extend(["gio".into(), "open".into(), path.into_os_string()]);
    let refs: Vec<_> = argv.iter().map(|s| s.as_os_str()).collect();
    let process = gio::Subprocess::newv(&refs, gio::SubprocessFlags::NONE).map_err(error)?;
    process.wait_check_async(gio::Cancellable::NONE, |r| {
        if let Err(e) = r {
            log::warn!("Opening file failed: {e}");
        }
    });
    Ok(())
}
/// Transfers never overwrite an existing target or follow directory symlinks.
pub fn transfer(path: &str, destination: &str, moving: bool) -> Result<(), ErrorBody> {
    let source = absolute(path)?;
    let canonical = source.canonicalize().map_err(error)?;
    if canonical == Path::new("/") || (moving && canonical == glib::home_dir()) {
        return Err(error("This location cannot be moved or copied."));
    }
    let folder = directory_path(destination)?;
    let meta = std::fs::symlink_metadata(&source).map_err(error)?;
    if meta.is_dir() && folder.starts_with(&canonical) {
        return Err(error("Choose a destination outside this folder."));
    }
    let target = folder.join(source.file_name().ok_or_else(|| error("Choose a file or folder"))?);
    if std::fs::symlink_metadata(&target).is_ok() {
        return Err(error("An item with this name already exists at the destination."));
    }
    if moving {
        gio::File::for_path(&source)
            .move_(&gio::File::for_path(target), gio::FileCopyFlags::NOFOLLOW_SYMLINKS, gio::Cancellable::NONE, None)
            .map_err(error)
    } else {
        copy_tree(&source, &target)
    }
}
fn copy_tree(source: &Path, target: &Path) -> Result<(), ErrorBody> {
    let metadata = std::fs::symlink_metadata(source).map_err(error)?;
    if metadata.is_dir() {
        std::fs::create_dir(target).map_err(error)?;
        for entry in std::fs::read_dir(source).map_err(error)? {
            let entry = entry.map_err(error)?;
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if !metadata.is_file() && !metadata.file_type().is_symlink() {
            return Err(error("Special files cannot be copied."));
        }
        gio::File::for_path(source)
            .copy(&gio::File::for_path(target), gio::FileCopyFlags::NOFOLLOW_SYMLINKS, gio::Cancellable::NONE, None)
            .map_err(error)
    }
}
pub fn thumbnail(path: &str) -> Result<Option<String>, ErrorBody> {
    use gtk::gdk_pixbuf::Pixbuf;
    let path = absolute(path)?;
    let Ok(meta) = std::fs::metadata(&path) else { return Ok(None) };
    if !meta.is_file() || meta.len() > 20 * 1024 * 1024 {
        return Ok(None);
    }
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !["png", "jpg", "jpeg", "webp", "gif", "bmp"].contains(&extension.as_str()) {
        return Ok(None);
    }
    let Some((_, width, height)) = Pixbuf::file_info(&path) else { return Ok(None) };
    if width <= 0 || height <= 0 || i64::from(width) * i64::from(height) > 64_000_000 {
        return Ok(None);
    }
    let Ok(image) = Pixbuf::from_file_at_scale(&path, 160, 120, true) else { return Ok(None) };
    let bytes = image.save_to_bufferv("png", &[]).map_err(error)?;
    Ok(Some(format!("data:image/png;base64,{}", glib::base64_encode(&bytes))))
}

static BOOKMARK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn bookmarks() -> Vec<FileLocation> {
    let path = glib::user_config_dir().join("meridian/orbit-bookmarks.json");
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|s| Path::new(s).is_absolute())
        .map(|path| FileLocation {
            name: Path::new(&path).file_name().map_or_else(|| "/".into(), |s| s.to_string_lossy().into_owned()),
            path,
        })
        .collect()
}
pub fn bookmark(path: &str) -> Result<Vec<FileLocation>, ErrorBody> {
    let _guard = BOOKMARK_LOCK.lock().map_err(error)?;
    let requested = absolute(path)?;
    let mut saved = bookmarks().into_iter().map(|l| l.path).collect::<Vec<_>>();
    if saved.iter().any(|s| Path::new(s) == requested) {
        saved.retain(|s| Path::new(s) != requested);
    } else {
        let path = directory_path(path)?.to_string_lossy().into_owned();
        if !saved.contains(&path) {
            saved.push(path);
        }
    }
    let dir = glib::user_config_dir().join("meridian");
    std::fs::create_dir_all(&dir).map_err(error)?;
    let temp = dir.join("orbit-bookmarks.json.tmp");
    std::fs::write(&temp, serde_json::to_vec(&saved).map_err(error)?).map_err(error)?;
    std::fs::rename(temp, dir.join("orbit-bookmarks.json")).map_err(error)?;
    Ok(bookmarks())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn properties_inspect_files_folders_and_broken_links_without_following_links() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = std::env::temp_dir().join(format!("meridian-properties-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("notes.txt");
        std::fs::write(&file, "hello").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
        let result = properties(file.to_str().unwrap()).unwrap();
        assert_eq!(result.size, Some(5.));
        assert_eq!(result.name, "notes.txt");
        assert_eq!(result.permissions, "rw-r----- (640)");
        assert!(result.modified.is_some());
        assert!(properties(root.to_str().unwrap()).unwrap().size.is_none());
        let link = root.join("shortcut");
        symlink("missing-target", &link).unwrap();
        let result = properties(link.to_str().unwrap()).unwrap();
        assert_eq!(result.kind, "Symbolic link");
        assert_eq!(result.link_target.as_deref(), Some("missing-target"));
        assert!(properties("relative").is_err());
        assert!(properties("/tmp/../etc/passwd").is_err());
        assert!(properties(root.join("absent").to_str().unwrap()).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn transfers_directories_and_symlinks_without_overwriting() {
        let root = std::env::temp_dir().join(format!("meridian-transfer-{}", std::process::id()));
        let source = root.join("source");
        let target = root.join("target");
        let moved = root.join("moved");
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&moved).unwrap();
        std::fs::write(source.join("nested/file.txt"), "keep").unwrap();
        std::os::unix::fs::symlink("nested/file.txt", source.join("link")).unwrap();
        transfer(source.to_str().unwrap(), target.to_str().unwrap(), false).unwrap();
        assert_eq!(std::fs::read_to_string(target.join("source/nested/file.txt")).unwrap(), "keep");
        assert!(target.join("source/link").is_symlink());
        assert!(transfer(source.to_str().unwrap(), target.to_str().unwrap(), false).is_err());
        assert!(transfer(source.to_str().unwrap(), source.join("nested").to_str().unwrap(), false).is_err());
        assert!(source.join("nested/file.txt").exists());
        transfer(source.to_str().unwrap(), moved.to_str().unwrap(), true).unwrap();
        assert!(!source.exists());
        assert!(moved.join("source/nested/file.txt").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn thumbnail_does_not_try_to_decode_non_images() {
        let root = std::env::temp_dir().join(format!("meridian-thumbnail-validation-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("file.txt");
        std::fs::write(&path, "text").unwrap();
        assert!(thumbnail(path.to_str().unwrap()).unwrap().is_none());
        assert!(thumbnail(root.to_str().unwrap()).unwrap().is_none());
        assert!(thumbnail("relative.png").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "Requires a desktop D-Bus connection for the system image loaders"]
    fn thumbnails_decode_images_and_reject_other_files() {
        use gtk::gdk_pixbuf::{Colorspace, Pixbuf};
        let root = std::env::temp_dir().join(format!("meridian-thumbs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let image = Pixbuf::new(Colorspace::Rgb, false, 8, 320, 240).unwrap();
        image.fill(0x668ed8ff);
        let path = root.join("image.png");
        image.savev(&path, "png", &[]).unwrap();
        assert!(thumbnail(path.to_str().unwrap()).unwrap().unwrap().starts_with("data:image/png;base64,"));
        std::fs::write(root.join("file.txt"), "plain text").unwrap();
        assert!(thumbnail(root.join("file.txt").to_str().unwrap()).unwrap().is_none());
        std::fs::write(root.join("invalid.png"), "not an image").unwrap();
        assert!(thumbnail(root.join("invalid.png").to_str().unwrap()).unwrap().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn manages_real_files_without_overwriting_or_accepting_path_names() {
        let root = std::env::temp_dir().join(format!("meridian-files-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.to_str().unwrap();
        mkdir(path, "Documents").unwrap();
        mkdir(path, ".hidden").unwrap();
        assert_eq!(list(path, false).unwrap().entries.len(), 1);
        assert_eq!(list(path, true).unwrap().entries.len(), 2);
        assert!(mkdir(path, "../escape").is_err());
        assert!(rename(root.join("Documents").to_str().unwrap(), ".hidden").is_err());
        rename(root.join("Documents").to_str().unwrap(), "Renamed").unwrap();
        assert!(root.join("Renamed").is_dir());
        assert!(absolute("relative").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
