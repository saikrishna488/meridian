//! Transfers publish complete staged trees with Linux RENAME_NOREPLACE.
//! A cross-device move first holds the source under a private sibling directory;
//! failures restore it, and a crash leaves a manifest alongside recoverable data.
use meridian_protocol::{ErrorBody, ErrorCode, FileTransfer, TransferProgress};
use std::{
    collections::HashMap,
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

fn error(message: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, message.to_string())
}
fn io_error(e: std::io::Error) -> ErrorBody {
    if e.raw_os_error() == Some(libc::ENOSPC) || e.raw_os_error() == Some(libc::EDQUOT) {
        error("Not enough storage space. No incomplete destination was published.")
    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
        error("Permission denied. Check access to the source and destination folders.")
    } else {
        error(e)
    }
}

#[derive(Default)]
pub struct Control {
    cancel: AtomicBool,
    #[cfg(test)]
    fail_after: std::sync::atomic::AtomicU64,
    #[cfg(test)]
    before_publish: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    progress: Mutex<(u64, u64, u64)>, // copied bytes, total bytes, completed entries
}
impl Control {
    fn check(&self) -> Result<(), ErrorBody> {
        #[cfg(test)]
        if self.fail_after.load(Ordering::Relaxed) > 0
            && self.progress.lock().unwrap().0 >= self.fail_after.load(Ordering::Relaxed)
        {
            return Err(error("Injected write failure"));
        }
        if self.cancel.load(Ordering::Relaxed) {
            Err(error("Transfer cancelled. The original data was preserved."))
        } else {
            Ok(())
        }
    }
    fn update(&self, bytes: u64, entry: bool) {
        if let Ok(mut p) = self.progress.lock() {
            p.0 += bytes;
            p.2 += u64::from(entry);
        }
    }
}

pub fn rename_noreplace(source: &Path, target: &Path) -> std::io::Result<()> {
    let source = CString::new(source.as_os_str().as_bytes())?;
    let target = CString::new(target.as_os_str().as_bytes())?;
    // SAFETY: both strings are NUL-terminated; paths are never interpreted as code.
    let result = unsafe {
        libc::renameat2(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, target.as_ptr(), libc::RENAME_NOREPLACE)
    };
    if result == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

struct Stage {
    directory: PathBuf,
    preserve: bool,
}
impl Stage {
    fn new(parent: &Path, original: &Path, kind: &str) -> Result<Self, ErrorBody> {
        let mut pattern = parent.join(".meridian-transfer-XXXXXX").as_os_str().as_bytes().to_vec();
        pattern.push(0);
        // SAFETY: mkdtemp mutates this writable, NUL-terminated template in place.
        if unsafe { libc::mkdtemp(pattern.as_mut_ptr().cast()) }.is_null() {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        pattern.pop();
        let stage = Self { directory: PathBuf::from(std::ffi::OsStr::from_bytes(&pattern)), preserve: false };
        let manifest = serde_json::json!({"version":1, "operation":kind, "original":original, "data":"data", "recovery":"Never delete data from a held move before restoring or verifying it."});
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(stage.directory.join("recovery.json"))
            .map_err(io_error)?;
        file.write_all(&serde_json::to_vec(&manifest).map_err(error)?).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        File::open(&stage.directory).and_then(|d| d.sync_all()).map_err(io_error)?;
        Ok(stage)
    }
    fn data(&self) -> PathBuf {
        self.directory.join("data")
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        // A held source must survive unwinding, I/O failures and failed restores.
        if self.preserve && fs::symlink_metadata(self.data()).is_ok() {
            log::error!("component=files event=held_source_preserved recovery={:?}", self.directory);
            return;
        }
        // Only private staging trees created by this operation are removed.
        if let Err(e) = fs::remove_dir_all(&self.directory) {
            log::warn!("component=files event=stage_cleanup_failed code={:?}", e.kind());
        }
    }
}

fn size(source: &Path, control: &Control, depth: usize, entries: &mut usize) -> Result<u64, ErrorBody> {
    control.check()?;
    *entries += 1;
    if depth > 128 || *entries > 200_000 {
        return Err(error("This transfer exceeds the safety limit of 200,000 entries or 128 folder levels."));
    }
    let metadata = fs::symlink_metadata(source).map_err(io_error)?;
    if metadata.is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(error("Special files cannot be copied."));
    }
    let mut total = 0u64;
    for entry in fs::read_dir(source).map_err(io_error)? {
        total = total
            .checked_add(size(&entry.map_err(io_error)?.path(), control, depth + 1, entries)?)
            .ok_or_else(|| error("Transfer is too large."))?;
    }
    Ok(total)
}
fn same_file(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.len() == after.len()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
}
fn copy(source: &Path, target: &Path, control: &Control, depth: usize, entries: &mut usize) -> Result<(), ErrorBody> {
    control.check()?;
    *entries += 1;
    if depth > 128 || *entries > 200_000 {
        return Err(error("The source grew beyond the transfer safety limit."));
    }
    let metadata = fs::symlink_metadata(source).map_err(io_error)?;
    if metadata.is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(source).map_err(io_error)?, target).map_err(io_error)?;
    } else if metadata.is_dir() {
        fs::create_dir(target).map_err(io_error)?;
        for entry in fs::read_dir(source).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            copy(&entry.path(), &target.join(entry.file_name()), control, depth + 1, entries)?;
        }
        if !same_file(&metadata, &fs::symlink_metadata(source).map_err(io_error)?) {
            return Err(error("The source folder changed during the transfer. Try again when it is idle."));
        }
        fs::set_permissions(target, fs::Permissions::from_mode(metadata.mode() & 0o777)).map_err(io_error)?;
        File::open(target).and_then(|d| d.sync_all()).map_err(io_error)?;
    } else if metadata.is_file() {
        let mut input = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(source)
            .map_err(io_error)?;
        if !same_file(&metadata, &input.metadata().map_err(io_error)?) {
            return Err(error("The source changed during the transfer."));
        }
        let mut output = OpenOptions::new().write(true).create_new(true).mode(0o600).open(target).map_err(io_error)?;
        let mut buffer = [0u8; 64 * 1024];
        let mut copied = 0u64;
        loop {
            control.check()?;
            let count = input.read(&mut buffer).map_err(io_error)?;
            if count == 0 {
                break;
            }
            copied = copied.saturating_add(count as u64);
            if copied > metadata.len() {
                return Err(error("The source file grew during the transfer. Try again when it is idle."));
            }
            output.write_all(&buffer[..count]).map_err(io_error)?;
            control.update(count as u64, false);
        }
        if !same_file(&metadata, &input.metadata().map_err(io_error)?) {
            return Err(error("The source file changed during the transfer. Try again when it is idle."));
        }
        output.set_permissions(fs::Permissions::from_mode(metadata.mode() & 0o777)).map_err(io_error)?;
        output.sync_all().map_err(io_error)?;
    } else {
        return Err(error("Special files cannot be copied."));
    }
    control.update(0, true);
    Ok(())
}
fn available(folder: &Path) -> Result<u64, ErrorBody> {
    let path = CString::new(folder.as_os_str().as_bytes()).map_err(error)?;
    let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: path is valid and statvfs initializes info on success.
    if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return Err(io_error(std::io::Error::last_os_error()));
    }
    let info = unsafe { info.assume_init() };
    Ok(info.f_bavail.saturating_mul(info.f_frsize))
}

pub fn run(choice: &FileTransfer, control: &Control) -> Result<(), ErrorBody> {
    let source = crate::files::absolute(&choice.path)?;
    let metadata = fs::symlink_metadata(&source).map_err(io_error)?;
    let canonical = if metadata.is_symlink() { source.clone() } else { source.canonicalize().map_err(io_error)? };
    if canonical == Path::new("/") || (choice.move_file && canonical == gtk::glib::home_dir()) {
        return Err(error("This location cannot be moved or copied."));
    }
    let folder = crate::files::directory_path(&choice.destination)?;
    if metadata.is_dir() && folder.starts_with(&canonical) {
        return Err(error("Choose a destination outside this folder."));
    }
    let target = folder.join(source.file_name().ok_or_else(|| error("Choose a file or folder."))?);
    control.check()?;
    if choice.move_file {
        match rename_noreplace(&source, &target) {
            Ok(()) => {
                File::open(&folder).and_then(|d| d.sync_all())
                    .and_then(|()| File::open(source.parent().unwrap()).and_then(|directory| directory.sync_all()))
                    .map_err(|e| error(format!("The move completed to {}, but durability could not be confirmed ({e}). Verify the destination before retrying.", target.display())))?;
                return Ok(());
            }
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {}
            Err(e) => return Err(io_error(e)),
        }
    }
    if fs::symlink_metadata(&target).is_ok() {
        return Err(error("An item with this name already exists at the destination."));
    }
    let total = size(&source, control, 0, &mut 0)?;
    if let Ok(mut p) = control.progress.lock() {
        p.1 = total;
    }
    if total > available(&folder)? {
        return Err(error("Not enough storage space at the destination."));
    }
    let stage = Stage::new(&folder, &source, "copy")?;
    // A held move has its own recovery manifest, written and synced before the
    // source is renamed. It is never discarded on an unsuccessful restore.
    let mut held = if choice.move_file {
        let mut held =
            Stage::new(source.parent().ok_or_else(|| error("Cannot move this location."))?, &source, "held-move")?;
        held.preserve = true;
        rename_noreplace(&source, &held.data()).map_err(io_error)?;
        Some(held)
    } else {
        None
    };
    let from = held.as_ref().map(Stage::data).unwrap_or_else(|| source.clone());
    let result = (|| {
        if let Some(held) = &held {
            File::open(&held.directory).and_then(|d| d.sync_all()).map_err(io_error)?;
            File::open(source.parent().unwrap()).and_then(|d| d.sync_all()).map_err(io_error)?;
        }
        copy(&from, &stage.data(), control, 0, &mut 0)?;
        control.check()?;
        #[cfg(test)]
        if let Some(hook) = control.before_publish.lock().unwrap().take() {
            hook();
        }
        rename_noreplace(&stage.data(), &target).map_err(io_error)?;
        File::open(&folder).and_then(|d| d.sync_all()).map_err(|e| error(format!("The destination was completed, but durability could not be confirmed: {e}. Verify it before removing any source.")))?;
        Ok(())
    })();
    if result.is_ok()
        && let Some(held) = &mut held
    {
        held.preserve = false;
    }
    if result.is_err()
        && let Some(held) = held
        && let Err(e) = rename_noreplace(&held.data(), &source)
    {
        let recovery = held.directory.clone();
        drop(held);
        return Err(error(format!(
            "The move could not finish or restore its source ({e}). Original data is preserved in {}. Do not delete this recovery folder.",
            recovery.display()
        )));
    }
    result
}

struct Job {
    control: Arc<Control>,
    result: Arc<Mutex<Option<Result<(), ErrorBody>>>>,
}
#[derive(Default)]
pub struct Transfers {
    jobs: HashMap<u32, Job>,
    next: u32,
}
impl Transfers {
    pub fn start(&mut self, choice: FileTransfer) -> Result<TransferProgress, ErrorBody> {
        if self.jobs.len() >= 32 {
            self.jobs.retain(|_, job| job.result.lock().map_or(true, |r| r.is_none()));
        }
        if self.jobs.values().filter(|job| job.result.lock().map_or(true, |r| r.is_none())).count() >= 2 {
            return Err(error("Two transfers are already running. Wait or cancel one."));
        }
        self.next = self.next.checked_add(1).ok_or_else(|| error("Transfer identifier limit reached."))?;
        let id = self.next;
        let control = Arc::new(Control::default());
        let result = Arc::new(Mutex::new(None));
        self.jobs.insert(id, Job { control: control.clone(), result: result.clone() });
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(|| run(&choice, &control)).unwrap_or_else(|_| {
                Err(error("Transfer worker failed; inspect recovery folders before retrying a move."))
            });
            if let Ok(mut result) = result.lock() {
                *result = Some(outcome);
            }
        });
        self.status(id)
    }
    pub fn has_active(&self) -> bool {
        self.jobs.values().any(|job| job.result.lock().map_or(true, |result| result.is_none()))
    }
    pub fn cancel_all(&self) {
        for job in self.jobs.values() {
            job.control.cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn list(&self) -> Vec<TransferProgress> {
        self.jobs.keys().filter_map(|id| self.status(*id).ok()).collect()
    }
    pub fn status(&self, id: u32) -> Result<TransferProgress, ErrorBody> {
        let job = self.jobs.get(&id).ok_or_else(|| error("This transfer is no longer available."))?;
        let (bytes, total, entries) = *job.control.progress.lock().map_err(error)?;
        let result = job.result.lock().map_err(error)?;
        Ok(TransferProgress {
            id,
            bytes: bytes as f64,
            total: total as f64,
            entries: entries as f64,
            done: result.is_some(),
            error: result.as_ref().and_then(|r| r.as_ref().err()).map(|e| e.message.clone()),
        })
    }
    pub fn cancel(&self, id: u32) -> Result<(), ErrorBody> {
        let job = self.jobs.get(&id).ok_or_else(|| error("This transfer is no longer available."))?;
        job.control.cancel.store(true, Ordering::Relaxed);
        Ok(())
    }
}
impl Drop for Transfers {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    struct Fixture(PathBuf);
    impl Fixture {
        fn at(parent: &Path) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = parent.join(format!(
                "meridian-transfer-check-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn new() -> Self {
            Self::at(&std::env::temp_dir())
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
        fn choice(&self, source: &Path, moving: bool) -> FileTransfer {
            FileTransfer {
                path: source.to_str().unwrap().into(),
                destination: self.0.to_str().unwrap().into(),
                move_file: moving,
            }
        }
        fn no_stages(&self) {
            assert!(
                !fs::read_dir(&self.0).unwrap().any(|e| e
                    .unwrap()
                    .file_name()
                    .as_bytes()
                    .starts_with(b".meridian-transfer-"))
            );
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn thousands_of_files_and_broken_symlinks_copy_completely() {
        let source = Fixture::new();
        let folder = source.path("tree");
        fs::create_dir(&folder).unwrap();
        for n in 0..2048 {
            fs::write(folder.join(n.to_string()), n.to_string()).unwrap();
        }
        std::os::unix::fs::symlink("missing", folder.join("broken")).unwrap();
        let destination = Fixture::new();
        let control = Control::default();
        run(&destination.choice(&folder, false), &control).unwrap();
        assert_eq!(fs::read_dir(destination.path("tree")).unwrap().count(), 2049);
        assert_eq!(fs::read_to_string(destination.path("tree/2047")).unwrap(), "2047");
        assert_eq!(fs::read_link(destination.path("tree/broken")).unwrap(), Path::new("missing"));
        assert_eq!(control.progress.lock().unwrap().2, 2050);
        destination.no_stages();
    }
    #[test]
    fn interrupted_copy_and_conflict_never_publish_partial_data() {
        let source = Fixture::new();
        let file = source.path("data");
        fs::write(&file, vec![1u8; 256 * 1024]).unwrap();
        let destination = Fixture::new();
        let control = Control::default();
        control.fail_after.store(64 * 1024, Ordering::Relaxed);
        assert!(run(&destination.choice(&file, false), &control).is_err());
        assert!(!destination.path("data").exists());
        assert_eq!(fs::metadata(&file).unwrap().len(), 256 * 1024);
        destination.no_stages();
        let control = Control::default();
        let target = destination.path("data");
        *control.before_publish.lock().unwrap() = Some(Box::new(move || {
            fs::write(target, "keep existing").unwrap();
        }));
        assert!(run(&destination.choice(&file, false), &control).is_err());
        assert_eq!(fs::read_to_string(destination.path("data")).unwrap(), "keep existing");
        destination.no_stages();
    }
    #[test]
    fn cancellation_during_copy_preserves_the_source_and_removes_staging() {
        let source = Fixture::new();
        let file = source.path("large");
        let output = File::create(&file).unwrap();
        output.set_len(128 * 1024 * 1024).unwrap();
        let destination = Fixture::new();
        let choice = destination.choice(&file, false);
        let control = Arc::new(Control::default());
        let worker = control.clone();
        let thread = std::thread::spawn(move || run(&choice, &worker));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while control.progress.lock().unwrap().0 == 0 && std::time::Instant::now() < until {
            std::thread::yield_now();
        }
        control.cancel.store(true, Ordering::Relaxed);
        assert!(thread.join().unwrap().is_err());
        assert!(file.exists());
        assert!(!destination.path("large").exists());
        destination.no_stages();
    }
    #[test]
    fn cross_filesystem_moves_restore_sources_on_failure_then_succeed() {
        let source = Fixture::new();
        let destination = Fixture::at(Path::new("/dev/shm"));
        if fs::metadata(&source.0).unwrap().dev() == fs::metadata(&destination.0).unwrap().dev() {
            return;
        }
        let folder = source.path("tree");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("data"), vec![7u8; 128 * 1024]).unwrap();
        let control = Control::default();
        control.fail_after.store(64 * 1024, Ordering::Relaxed);
        assert!(run(&destination.choice(&folder, true), &control).is_err());
        assert!(folder.join("data").exists());
        assert!(!destination.path("tree").exists());
        source.no_stages();
        destination.no_stages();
        run(&destination.choice(&folder, true), &Control::default()).unwrap();
        assert!(!folder.exists());
        assert_eq!(fs::metadata(destination.path("tree/data")).unwrap().len(), 128 * 1024);
        source.no_stages();
        destination.no_stages();
    }
    #[test]
    fn held_move_data_survives_unwinding() {
        let source = Fixture::new();
        let file = source.path("original");
        fs::write(&file, "original data").unwrap();
        let directory = std::cell::RefCell::new(PathBuf::new());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut held = Stage::new(&source.0, &file, "held-move").unwrap();
            held.preserve = true;
            *directory.borrow_mut() = held.directory.clone();
            rename_noreplace(&file, &held.data()).unwrap();
            panic!("simulate worker interruption");
        }));
        assert_eq!(fs::read_to_string(directory.borrow().join("data")).unwrap(), "original data");
        assert!(directory.borrow().join("recovery.json").exists());
        rename_noreplace(&directory.borrow().join("data"), &file).unwrap();
    }
    #[test]
    fn insufficient_space_is_detected_without_allocating_a_large_copy() {
        let source = Fixture::new();
        let destination = Fixture::new();
        let file = source.path("too-large");
        File::create(&file).unwrap().set_len(available(&destination.0).unwrap() + 1024 * 1024).unwrap();
        assert!(run(&destination.choice(&file, false), &Control::default()).unwrap_err().message.contains("storage"));
        assert!(file.exists());
        destination.no_stages();
    }
    #[test]
    fn special_files_are_rejected_before_any_copy_is_published() {
        let source = Fixture::new();
        let fifo = source.path("pipe");
        let cpath = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        let destination = Fixture::new();
        assert!(run(&destination.choice(&fifo, false), &Control::default()).is_err());
        destination.no_stages();
    }
}
