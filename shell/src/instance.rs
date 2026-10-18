//! Per-compositor locks also protect against duplicate shells on separate buses.
use std::{
    fs::{File, OpenOptions},
    hash::{Hash, Hasher},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

pub struct InstanceGuard {
    _file: File,
}
impl InstanceGuard {
    pub fn acquire(component: &str) -> std::io::Result<Self> {
        let runtime =
            std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| std::io::Error::other("XDG_RUNTIME_DIR is required"))?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        // Absolute and relative names for the same socket must share a lock.
        let display = std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into());
        let socket = if Path::new(&display).is_absolute() {
            std::path::PathBuf::from(display)
        } else {
            Path::new(&runtime).join(display)
        };
        socket.canonicalize()?.hash(&mut hash);
        Self::at(&Path::new(&runtime).join(format!("meridian-{component}-{:x}.lock", hash.finish())))
    }
    fn at(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let metadata = file.metadata()?;
        // SAFETY: geteuid has no preconditions; flock holds this open descriptor.
        if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err(std::io::Error::other("Invalid instance lock"));
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { _file: file })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicates_are_rejected_and_crash_style_drop_releases_the_lock() {
        let path = std::env::temp_dir().join(format!("meridian-instance-{}", std::process::id()));
        let first = InstanceGuard::at(&path).unwrap();
        assert!(InstanceGuard::at(&path).is_err());
        drop(first);
        drop(InstanceGuard::at(&path).unwrap());
        std::fs::remove_file(path).unwrap();
    }
}
