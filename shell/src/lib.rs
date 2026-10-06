//! Meridian's native UI host, shared by `meridian-shell` (the desktop) and
//! `meridian-greeter` (the login screen): layer-shell surfaces hosting
//! locked-down WebViews, the typed UI bridge, and the `meridian://` scheme.
//! See docs/ARCHITECTURE.md.

pub mod bridge;
pub mod greetd;
pub mod greeter;
pub mod paths;
pub mod scheme;
pub mod shell;
pub mod surface;

use gtk::gdk;
use gtk::prelude::*;

/// Checks that the environment can host Meridian surfaces.
pub fn preflight() -> Result<(), String> {
    let display = gdk::Display::default().ok_or("no display connection")?;
    if !display.type_().name().contains("Wayland") {
        return Err("Meridian requires a Wayland session".into());
    }
    if !gtk4_layer_shell::is_supported() {
        return Err("the compositor does not support wlr-layer-shell; run inside a layer-shell \
                    compositor (e.g. tools/dev-session.sh starts a nested labwc)"
            .into());
    }
    Ok(())
}

/// Surfaces are transparent: all pixels come from web content.
pub fn install_css() {
    let css = gtk::CssProvider::new();
    css.load_from_string("window.meridian-surface { background: transparent; }");
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
