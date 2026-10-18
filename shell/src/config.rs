//! Bounded reads and atomic, private configuration writes with corrupt backups.
use gtk::{gio, prelude::*};
use meridian_protocol::{ErrorBody, ErrorCode};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
const LIMIT: u64 = 4 * 1024 * 1024;
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
fn bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("Configuration must be a regular file."));
    }
    let mut data = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut data)?;
    if data.len() as u64 > LIMIT {
        return Err(std::io::Error::other("Configuration exceeds 4 MiB; preserve and repair it before saving."));
    }
    Ok(data)
}
pub fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&bytes(path).ok()?).ok()
}
pub fn save<T: Serialize + DeserializeOwned>(path: &Path, value: &T) -> Result<(), ErrorBody> {
    let parent = path.parent().ok_or_else(|| error("Configuration folder unavailable."))?;
    std::fs::create_dir_all(parent).map_err(error)?;
    match bytes(path) {
        Ok(data) if serde_json::from_slice::<T>(&data).is_err() => {
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(error)?.as_nanos();
            let backup = path.with_extension(format!("json.corrupt-{stamp}"));
            let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&backup).map_err(error)?;
            file.write_all(&data).map_err(error)?;
            file.sync_all().map_err(error)?;
            File::open(parent).and_then(|dir| dir.sync_all()).map_err(error)?;
            log::warn!("component=config event=corrupt_backup_saved");
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(error(e)),
    }
    let data = serde_json::to_vec_pretty(value).map_err(error)?;
    if data.len() as u64 > LIMIT {
        return Err(error("Configuration exceeds 4 MiB."));
    }
    gio::File::for_path(path)
        .replace_contents(
            &data,
            None,
            false,
            gio::FileCreateFlags::PRIVATE | gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::Cancellable::NONE,
        )
        .map_err(error)?;
    File::open(parent).and_then(|dir| dir.sync_all()).map_err(error)
}
/// Versioned stores accept legacy plain JSON and migrate on the next save.
/// Future formats are deliberately read-only for older Meridian versions.
#[derive(Serialize)]
struct Versioned<T> {
    schema_version: u32,
    data: T,
}
impl<'de, T: DeserializeOwned> serde::Deserialize<'de> for Versioned<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let data = if let Some(version) = value.get("schema_version") {
            if version.as_u64() != Some(1) {
                return Err(serde::de::Error::custom("Unsupported configuration version"));
            }
            value.get("data").cloned().ok_or_else(|| serde::de::Error::custom("Missing configuration data"))?
        } else {
            value
        };
        Ok(Self { schema_version: 1, data: serde_json::from_value(data).map_err(serde::de::Error::custom)? })
    }
}
pub fn read_versioned<T: DeserializeOwned>(path: &Path) -> Option<T> {
    read::<Versioned<T>>(path).map(|stored| stored.data)
}
pub fn save_versioned<T: Serialize + DeserializeOwned + Clone>(path: &Path, value: &T) -> Result<(), ErrorBody> {
    if let Ok(data) = bytes(path)
        && let Ok(existing) = serde_json::from_slice::<serde_json::Value>(&data)
        && let Some(version) = existing.get("schema_version")
        && version.as_u64() != Some(1)
    {
        return Err(error("This configuration uses an unsupported version. Upgrade Meridian before saving."));
    }
    save(path, &Versioned { schema_version: 1, data: value.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_configuration_migrates_and_future_versions_are_preserved() {
        let root = std::env::temp_dir().join(format!("meridian-migration-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.json");
        std::fs::write(&path, r#"["legacy"]"#).unwrap();
        let value = read_versioned::<Vec<String>>(&path).unwrap();
        save_versioned(&path, &value).unwrap();
        let encoded: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(encoded["schema_version"], 1);
        assert_eq!(read_versioned::<Vec<String>>(&path).unwrap(), value);
        let future = br#"{"schema_version":2,"data":["future"]}"#;
        std::fs::write(&path, future).unwrap();
        assert!(read_versioned::<Vec<String>>(&path).is_none());
        assert!(save_versioned(&path, &value).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), future);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupted_configuration_is_preserved_before_atomic_replacement() {
        let root = std::env::temp_dir().join(format!("meridian-config-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.json");
        std::fs::write(&path, b"{interrupted").unwrap();
        assert!(read::<Vec<String>>(&path).is_none());
        save(&path, &vec!["recovered".to_owned()]).unwrap();
        assert_eq!(read::<Vec<String>>(&path).unwrap(), ["recovered"]);
        let backup = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().path()).find(|p| p != &path).unwrap();
        assert_eq!(std::fs::read(backup).unwrap(), b"{interrupted");
        File::create(&path).unwrap().set_len(LIMIT + 1).unwrap();
        assert!(read::<Vec<String>>(&path).is_none());
        assert!(save(&path, &vec!["never overwrite".to_owned()]).is_err());
        assert_eq!(std::fs::metadata(&path).unwrap().len(), LIMIT + 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
