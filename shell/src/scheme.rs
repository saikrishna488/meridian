//! The `meridian://` URI scheme: the only origin web content can load from.
//!
//! - `meridian://ui/<path>`      → file under the UI root (built `ui/dist`)
//! - `meridian://assets/<path>`  → file under the assets root
//!
//! The UI never supplies a filesystem path that is opened as-is: paths are
//! confined to their root (see [`resolve_under`]).

use std::path::{Component, Path, PathBuf};

use gtk::prelude::*;
use gtk::{gio, glib};
use webkit6::URISchemeRequest;

pub const SCHEME: &str = "meridian";

pub struct Roots {
    pub ui: PathBuf,
    pub assets: PathBuf,
}

pub struct SchemeHandler {
    roots: Roots,
    catalog: Option<std::rc::Rc<std::cell::RefCell<meridian_services::apps::AppCatalog>>>,
}

impl SchemeHandler {
    pub fn check_locker(&self) -> Result<(), String> {
        for path in [
            "surfaces/locker/index.html",
            "surfaces/locker/locker.js",
            "surfaces/locker/locker.css",
            "lib/base.css",
            "lib/wallpaper.css",
        ] {
            let file = resolve_under(&self.roots.ui, path).map_err(|_| {
                format!(
                    "Lock screen file {path} is missing. Rebuild the UI with npm run build in ui/ and restart Meridian."
                )
            })?;
            read_file(&file).map_err(|e| format!("Cannot read lock screen file {path}: {e}"))?;
        }
        Ok(())
    }

    pub fn new(roots: Roots) -> Self {
        SchemeHandler { roots, catalog: None }
    }

    pub fn with_catalog(
        mut self,
        catalog: std::rc::Rc<std::cell::RefCell<meridian_services::apps::AppCatalog>>,
    ) -> Self {
        self.catalog = Some(catalog);
        self
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
            "desktop" if path == "/wallpaper" => {
                read_file(&crate::desktop::custom_wallpaper().ok_or_else(|| not_found("no custom wallpaper"))?)
            }
            "app-icons" => {
                let id = glib::uri_unescape_string(path.trim_start_matches('/'), None::<&str>)
                    .ok_or_else(|| not_found("invalid application"))?;
                let catalog = self.catalog.as_ref().ok_or_else(|| not_found("no application catalog"))?.borrow();
                let app = catalog.get(&id).ok_or_else(|| not_found("unknown application"))?;
                let value = app.icon.as_deref().ok_or_else(|| not_found("application has no icon"))?;
                let icon = gio::Icon::for_string(value).map_err(|_| not_found("invalid application icon"))?;
                if let Some(file) = icon.downcast_ref::<gio::FileIcon>() {
                    let mut path = file.file().path().ok_or_else(|| not_found("nonlocal icon"))?;
                    if app.source.as_ref().is_some_and(|p| p.starts_with("/run/host"))
                        && path.is_absolute()
                        && !path.starts_with("/home")
                    {
                        let host_path = Path::new("/run/host").join(path.strip_prefix("/").unwrap());
                        if host_path.is_file() {
                            path = host_path;
                        }
                    }
                    // The path comes from an installed desktop entry, never a UI path.
                    if !mime_type(&path).starts_with("image/") {
                        return Err(forbidden("not an image"));
                    }
                    return read_file(&path);
                }
                let theme = gtk::IconTheme::new();
                let display = gtk::gdk::Display::default().ok_or_else(|| not_found("no display"))?;
                let mut paths = gtk::IconTheme::for_display(&display).search_path();
                paths.extend([
                    PathBuf::from("/run/host/usr/share/icons"),
                    PathBuf::from("/run/host/usr/share/pixmaps"),
                    PathBuf::from("/run/host/var/lib/flatpak/exports/share/icons"),
                ]);
                let paths: Vec<_> = paths.iter().map(PathBuf::as_path).collect();
                theme.set_search_path(&paths);
                theme.set_theme_name(Some("hicolor"));
                let paintable =
                    theme.lookup_by_gicon(&icon, 128, 2, gtk::TextDirection::None, gtk::IconLookupFlags::empty());
                let file = paintable.file().and_then(|f| f.path()).ok_or_else(|| not_found("no app icon"))?;
                read_file(&file)
            }
            // Serve Meridian's fixed icons from the UI's own origin.
            "ui" if path.starts_with("/desktop-icons/") => read_file(&resolve_under(&self.roots.assets, &path)?),
            "ui" => read_file(&resolve_under(&self.roots.ui, &path)?),
            "icons" => {
                let value = glib::uri_unescape_string(path.trim_start_matches('/'), None::<&str>)
                    .ok_or_else(|| not_found("invalid icon"))?;
                let icon = gio::Icon::for_string(&value).map_err(|_| not_found("invalid icon"))?;
                // Only theme icons are exposed; arbitrary file icons cannot read files.
                let themed = icon.downcast::<gio::ThemedIcon>().map_err(|_| forbidden("not a theme icon"))?;
                let display = gtk::gdk::Display::default().ok_or_else(|| not_found("no display"))?;
                let paintable = gtk::IconTheme::for_display(&display).lookup_by_gicon(
                    &themed,
                    96,
                    1,
                    gtk::TextDirection::None,
                    gtk::IconLookupFlags::empty(),
                );
                let file = paintable.file().and_then(|f| f.path()).ok_or_else(|| not_found("no icon"))?;
                read_file(&file)
            }
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
    fn locker_preflight_rejects_incomplete_bundles() {
        let ui = tmp_root();
        let handler = SchemeHandler::new(Roots { ui: ui.clone(), assets: ui });
        assert!(handler.check_locker().unwrap_err().contains("surfaces/locker/index.html"));
        let ui = tmp_root().join("complete-locker");
        for file in [
            "surfaces/locker/index.html",
            "surfaces/locker/locker.js",
            "surfaces/locker/locker.css",
            "lib/base.css",
            "lib/wallpaper.css",
        ] {
            let file = ui.join(file);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, "fixture").unwrap();
        }
        let ui = ui.canonicalize().unwrap();
        assert!(SchemeHandler::new(Roots { ui: ui.clone(), assets: ui }).check_locker().is_ok());
    }

    #[test]
    fn fixed_system_icon_assets_are_available_under_the_ui_origin() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets").canonicalize().unwrap();
        let handler = SchemeHandler::new(Roots { ui: tmp_root(), assets });
        let (icon, mime) = handler.respond("meridian://ui/desktop-icons/meridian/settings.svg").unwrap();
        assert_eq!(mime, "image/svg+xml");
        assert!(icon.as_ref().starts_with(b"<svg"));
        let (icon, _) = handler.respond("meridian://ui/desktop-icons/meridian/control-center.svg").unwrap();
        assert!(icon.as_ref().starts_with(b"<svg"));
        assert!(handler.respond("meridian://ui/desktop-icons/../../etc/passwd").is_err());
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
