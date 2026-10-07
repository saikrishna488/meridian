//! Meridian's native UI host, shared by `meridian-shell` (the desktop) and
//! `meridian-greeter` (the login screen): layer-shell surfaces hosting
//! locked-down WebViews, the typed UI bridge, and the `meridian://` scheme.
//! See docs/ARCHITECTURE.md.

mod bluetooth_agent;
pub mod bridge;
mod desktop;
mod files;
pub mod greetd;
pub mod greeter;
pub mod lock_auth;
pub mod paths;
pub mod power;
pub mod scheme;
mod settings_devices;
pub mod shell;
pub mod surface;
mod terminal;
mod uninstall;

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
    if !gtk4_session_lock::is_supported() {
        log::warn!("the compositor does not support ext-session-lock-v1; Lock will fail (see ADR-0007)");
    }
    Ok(())
}

/// Surfaces are transparent: all pixels come from web content.
pub fn install_css() {
    let css = gtk::CssProvider::new();
    css.load_from_string(
        "window.meridian-surface { background: transparent; }
        window.meridian-preferences { background: #f5f5f7; color: #242428; }
        window.meridian-preferences headerbar { background: #e8e9ee; color: #242428; }
        window.meridian-terminal headerbar { background: #24272d; color: #d6d9e0; }
        .meridian-window-button { min-width: 16px; min-height: 16px; padding: 0; border-radius: 50%; border: 1px solid rgba(0,0,0,.12); font-family: sans-serif; font-size: 12px; font-weight: bold; color: rgba(0,0,0,.65); box-shadow: none; }
        window.meridian-preferences.meridian-dark { background: #25272c; color: #e4e6ec; }
        window.meridian-preferences.meridian-dark headerbar, window.meridian-finder.meridian-dark headerbar { background: #303238; color: #e4e6ec; }
        window.meridian-terminal:not(.meridian-dark) headerbar { background: #eceef2; color: #242830; }
        .meridian-window-button.close { background: #ef726e; }
        .meridian-window-button.minimize { background: #e9bd59; }
        .meridian-window-button.maximize { background: #68bd85; }
        .meridian-window-button.close:hover { background: #f58c86; }
        .meridian-window-button.minimize:hover { background: #f1cd78; }
        .meridian-window-button.maximize:hover { background: #83cf9d; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
