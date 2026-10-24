//! Meridian's native UI host, shared by `meridian-shell` (the desktop) and
//! `meridian-greeter` (the login screen): layer-shell surfaces hosting
//! locked-down WebViews, the typed UI bridge, and the `meridian://` scheme.
//! See docs/ARCHITECTURE.md.

mod audio_devices;
mod bluetooth_agent;
pub mod bridge;
mod cache;
mod calendar;
mod config;
mod desktop;
mod files;
pub mod greetd;
pub mod greeter;
pub mod instance;
pub mod lock_auth;
mod notifications;
pub mod paths;
pub mod power;
pub mod power_settings;
mod pulse;
mod recovery;
pub mod scheme;
mod search;
mod settings_devices;
pub mod shell;
pub mod startup;
pub mod surface;
mod terminal;
mod transfer;
mod trash;
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
        .meridian-recovery { background: #171e2a; color: #edf2fb; padding: 40px; }
        window.meridian-preferences { background: #ffffff; color: #182338; }
        window.meridian-preferences headerbar { min-height: 36px; padding: 0 0 0 8px; background: #f4f6fa; color: #182338; box-shadow: inset 0 -1px #dce0e7; }
        window.meridian-preferences.meridian-dark { background: #171e2a; color: #edf2fb; }
        window.meridian-preferences.meridian-dark headerbar { background: #171e2a; color: #edf2fb; box-shadow: inset 0 -1px #303d51; }
        .meridian-window-button { min-width: 40px; min-height: 32px; padding: 0; border-radius: 0; border: 0; background: transparent; color: inherit; box-shadow: none; }
        .meridian-window-button:hover { background: rgba(100,110,130,.16); }
        .meridian-window-button.close:hover { background: #c52b38; color: #ffffff; }
        .meridian-window-button:focus-visible { outline: 2px solid #3578d5; outline-offset: -3px; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}

mod device_manager;
mod drivers;

mod users;

mod updates;

/// Shared chrome for the standalone Meridian applications.
pub fn install_app_css() {
    let css = gtk::CssProvider::new();
    css.load_from_string(include_str!("app.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
