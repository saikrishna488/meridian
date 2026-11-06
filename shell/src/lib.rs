//! Meridian's native UI host, shared by `meridian-shell` (the desktop) and
//! `meridian-greeter` (the login screen): layer-shell surfaces hosting
//! locked-down WebViews, the typed UI bridge, and the `meridian://` scheme.
//! See docs/ARCHITECTURE.md.

pub mod workflows;
mod audio_devices;
mod bluetooth_agent;
pub mod bridge;
mod cache;
mod calendar;
mod config;
mod default_apps;
mod desktop;
mod files;
mod gpu_monitor;
pub mod greetd;
pub mod greeter;
mod input_preferences;
pub mod instance;
pub mod lock_auth;
mod notifications;
pub mod paths;
pub mod power;
pub mod power_settings;
mod pulse;
mod recovery;
pub mod rendering;
pub mod scheme;
mod screenshot;
mod search;
mod settings_devices;
pub mod shell;
pub mod startup;
pub mod surface;
mod system_integrations;
mod terminal;
pub mod terminal_supervisor;
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
        window.meridian-preferences *:focus-visible, window.meridian-surface *:focus-visible { outline: 2px solid #00CC00; outline-offset: 2px; }
        .meridian-recovery { background: #f3f4f6; color: #20272b; padding: 40px; border: 1px solid #d6dcdf; }
        window.meridian-preferences { background: #f3f4f6; color: #20272b; border: 1px solid #d6dcdf; }
        window.meridian-preferences headerbar { min-height: 44px; padding: 4px 12px; background: #f3f4f6; color: #20272b; border-bottom: 1px solid #d6dcdf; box-shadow: none; }
        window.meridian-preferences.meridian-dark { background: #15191c; color: #edf2ef; }
        window.meridian-preferences.meridian-dark headerbar { background: #1b2024; color: #edf2ef; border-bottom: 1px solid #45524a; box-shadow: none; }
        .meridian-dark .meridian-window-button { color: #b2beb7; }
        .meridian-dark .meridian-window-button:hover { background-color: #00CC00; color: #061006; }
        .meridian-dark .meridian-recovery { background: #15191c; color: #edf2ef; border-color: #45524a; }
        .meridian-window-controls { margin: 0 12px 0 0; padding: 0; }
        .meridian-window-button { min-width: 32px; min-height: 32px; padding: 0; margin: 0; border-radius: 6px; border: 1px solid transparent; box-shadow: none; background-image: none; transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease; }
        .meridian-window-button { background-color: transparent; color: #606b70; }
        .meridian-window-button:hover { background-color: #00CC00; border-color: #00CC00; color: #061006; }
        .meridian-window-button:active { background-color: #00b300; border-color: #00CC00; color: #061006; }
        .meridian-window-button:disabled { opacity: 0.45; }
        .meridian-window-button:focus-visible { outline: 2px solid #00CC00; outline-offset: 2px; }
        .meridian-window-button:focus:not(:focus-visible) { outline: none; box-shadow: none; }",
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
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(false);
        settings.set_gtk_decoration_layout(Some(if crate::desktop::get().layout.window_buttons == "right" { ":close,minimize,maximize" } else { "close,minimize,maximize:" }));
    }
    let css = gtk::CssProvider::new();
    css.load_from_string(include_str!("app.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

mod capabilities;
mod storage;
