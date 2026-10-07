//! Finder operations. Blocking filesystem work stays off the GTK main loop.
//! Renames never overwrite, and removal always uses the system trash.
use gtk::prelude::*;
use gtk::{gio, glib};
use meridian_protocol::{ErrorBody, ErrorCode, FileEntry, FileListing, FileLocation};
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
    if let Ok(mounts) = std::fs::read_dir(format!("/run/media/{}", glib::user_name().to_string_lossy())) {
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
#[cfg(test)]
mod tests {
    use super::*;
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
