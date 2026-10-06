//! The `meridian://` URI scheme: the only origin web content can load from.
//!
//! - `meridian://ui/<path>`      → file under the UI root (built `ui/dist`)
//! - `meridian://assets/<path>`  → file under the assets root
//!
//! The UI never supplies a filesystem path that is opened as-is: paths are
//! confined to their root (see [`resolve_under`]).

use std::path::{Component, Path, PathBuf};

use gtk::{gio, glib};
use webkit6::URISchemeRequest;

pub const SCHEME: &str = "meridian";

pub struct Roots {
    pub ui: PathBuf,
    pub assets: PathBuf,
}

pub struct SchemeHandler {
    roots: Roots,
}

impl SchemeHandler {
    pub fn new(roots: Roots) -> Self {
        SchemeHandler { roots }
    }

    pub fn handle(&self, request: &URISchemeRequest) {
        let uri = request.uri().map(|u| u.to_string()).unwrap_or_default();
        match self.respond(&uri) {
            Ok((bytes, mime)) => {
                let len = bytes.len() as i64;
                let stream = gio::MemoryInputStream::from_bytes(&bytes);
                request.finish(&stream, len, Some(mime));
            }
            Err(e) => {
                log::warn!("{uri}: {}", e.message());
                let mut e = e;
                request.finish_error(&mut e);
            }
        }
    }

    fn respond(&self, uri: &str) -> Result<(glib::Bytes, &'static str), glib::Error> {
        let parsed = glib::Uri::parse(uri, glib::UriFlags::NONE).map_err(|e| not_found(&e.to_string()))?;
        let host = parsed.host().map(|h| h.to_string()).unwrap_or_default();
        let path = parsed.path().to_string();
        match host.as_str() {
            "ui" => read_file(&resolve_under(&self.roots.ui, &path)?),
            "assets" => read_file(&resolve_under(&self.roots.assets, &path)?),
            _ => Err(not_found("unknown host")),
        }
    }
}

/// Map a URI path onto a file under `root`, refusing anything that could
/// escape it: parent components, and symlinks pointing outside the root.
pub fn resolve_under(root: &Path, uri_path: &str) -> Result<PathBuf, glib::Error> {
    let relative = Path::new(uri_path.trim_start_matches('/'));
    let mut clean = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(forbidden("path escapes root"));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(not_found("empty path"));
    }
    let full = root.join(&clean).canonicalize().map_err(|_| not_found("no such file"))?;
    if !full.starts_with(root) {
        return Err(forbidden("path escapes root"));
    }
    Ok(full)
}

fn read_file(path: &Path) -> Result<(glib::Bytes, &'static str), glib::Error> {
    let data = std::fs::read(path).map_err(|e| not_found(&e.to_string()))?;
    Ok((glib::Bytes::from_owned(data), mime_type(path)))
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("js") | Some("mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("json") | Some("map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn not_found(msg: &str) -> glib::Error {
    glib::Error::new(gio::IOErrorEnum::NotFound, msg)
}

fn forbidden(msg: &str) -> glib::Error {
    glib::Error::new(gio::IOErrorEnum::PermissionDenied, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("meridian-scheme-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("home")).unwrap();
        std::fs::write(dir.join("home/index.html"), "<p>").unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn resolves_files_inside_root() {
        let root = tmp_root();
        assert_eq!(resolve_under(&root, "/home/index.html").unwrap(), root.join("home/index.html"));
        assert_eq!(resolve_under(&root, "/./home/index.html").unwrap(), root.join("home/index.html"));
    }

    #[test]
    fn rejects_traversal_and_missing() {
        let root = tmp_root();
        assert!(resolve_under(&root, "/../etc/passwd").is_err());
        assert!(resolve_under(&root, "/home/../../etc/passwd").is_err());
        assert!(resolve_under(&root, "/").is_err());
        assert!(resolve_under(&root, "/nope.html").is_err());
    }

    #[test]
    fn rejects_symlink_escape() {
        let root = tmp_root();
        let link = root.join("escape");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        assert!(resolve_under(&root, "/escape/hostname").is_err());
    }
}
