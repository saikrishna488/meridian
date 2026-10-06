//! Locations of the UI bundle and assets.

use std::path::PathBuf;

use crate::scheme::Roots;

/// Development defaults: the built UI and assets inside this repository.
const DEV_UI_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/dist");
const DEV_ASSETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

/// Installed locations, checked in order (see tools/install.sh).
const INSTALL_DIRS: [&str; 2] = ["/usr/local/share/meridian", "/usr/share/meridian"];

/// Where the UI bundle and assets are: `MERIDIAN_UI_DIR`/`MERIDIAN_ASSETS_DIR`
/// if set, else an installed copy, else this source tree (development).
pub fn roots() -> Result<Roots, String> {
    let installed = INSTALL_DIRS.iter().map(std::path::Path::new).find(|d| d.join("ui").is_dir());
    let (ui_default, assets_default) = match installed {
        Some(dir) => (dir.join("ui").display().to_string(), dir.join("assets").display().to_string()),
        None => (DEV_UI_DIR.to_owned(), DEV_ASSETS_DIR.to_owned()),
    };
    let ui = dir("MERIDIAN_UI_DIR", &ui_default)?;
    if !ui.join("surfaces/panel/index.html").is_file() {
        return Err(format!("UI bundle at {} is incomplete; build it with `npm run build` in ui/", ui.display()));
    }
    let assets = dir("MERIDIAN_ASSETS_DIR", &assets_default)?;
    Ok(Roots { ui, assets })
}

/// Resolve a directory from an environment variable or default, canonicalized
/// so the scheme handler's containment check compares real paths.
fn dir(var: &str, default: &str) -> Result<PathBuf, String> {
    let raw = std::env::var_os(var).map(PathBuf::from).unwrap_or_else(|| default.into());
    raw.canonicalize().map_err(|e| format!("{var}: cannot use {}: {e}", raw.display()))
}
