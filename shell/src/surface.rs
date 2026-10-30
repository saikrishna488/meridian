//! Layer-shell surfaces hosting locked-down WebViews.
//!
//! All GTK/WebKit specifics of hosting UI live here and in `bridge.rs`, so the
//! web engine can be swapped without touching the protocol or the UI.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, glib};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use meridian_protocol::{Event, SurfaceKind};
use webkit6::prelude::*;
use webkit6::{NetworkSession, PolicyDecisionType, UserContentManager, WebContext, WebView};

use crate::bridge::{self, Dispatch};
use crate::scheme::{SCHEME, SchemeHandler};

/// Shared resource handler; each surface receives an isolated context/session.
pub struct WebEnv {
    dev: bool,
    scheme: Rc<SchemeHandler>,
    theme_signal: Option<(gtk::IconTheme, glib::SignalHandlerId)>,
}

impl WebEnv {
    pub fn new(scheme: SchemeHandler, dev: bool) -> Self {
        let scheme = Rc::new(scheme);
        let theme_signal = gdk::Display::default().map(|display| {
            let weak = Rc::downgrade(&scheme);
            let theme = gtk::IconTheme::for_display(&display);
            let signal = theme.connect_changed(move |_| {
                if let Some(scheme) = weak.upgrade() {
                    scheme.invalidate_icons();
                }
            });
            (theme, signal)
        });
        WebEnv { dev, scheme, theme_signal }
    }

    pub fn invalidate_icons(&self) {
        self.scheme.invalidate_icons();
    }

    pub fn check_locker(&self) -> Result<(), String> {
        self.scheme.check_locker()
    }

    fn settings(&self) -> webkit6::Settings {
        let s = webkit6::Settings::new();
        let software_test =
            cfg!(feature = "integration-tests") && std::env::var("MERIDIAN_NESTED_TEST").as_deref() == Ok("1");
        s.set_hardware_acceleration_policy(if software_test {
            webkit6::HardwareAccelerationPolicy::Never
        } else {
            webkit6::HardwareAccelerationPolicy::Always
        });
        s.set_enable_developer_extras(self.dev);
        s.set_enable_back_forward_navigation_gestures(false);
        s.set_allow_file_access_from_file_urls(false);
        s.set_allow_universal_access_from_file_urls(false);
        s.set_javascript_can_open_windows_automatically(false);
        s.set_enable_page_cache(false);
        s.set_enable_media(false);
        s.set_enable_media_stream(false);
        s.set_enable_webrtc(false);
        s.set_enable_webaudio(false);
        s.set_enable_encrypted_media(false);
        s.set_enable_smooth_scrolling(true);
        s
    }

    fn create_view(&self, ucm: &UserContentManager) -> WebView {
        // Separate contexts and sessions isolate renderer crashes and storage.
        let context = WebContext::new();
        let security = context.security_manager().expect("WebContext always has a security manager");
        security.register_uri_scheme_as_secure(SCHEME);
        security.register_uri_scheme_as_cors_enabled(SCHEME);
        let handler = self.scheme.clone();
        context.register_uri_scheme(SCHEME, move |request| handler.handle(request));
        WebView::builder()
            .user_content_manager(ucm)
            .settings(&self.settings())
            .web_context(&context)
            .network_session(&NetworkSession::new_ephemeral())
            .build()
    }
}

impl Drop for WebEnv {
    fn drop(&mut self) {
        if let Some((theme, signal)) = self.theme_signal.take() {
            theme.disconnect(signal);
        }
    }
}

pub struct WebSurface {
    pub kind: SurfaceKind,
    pub window: gtk::Window,
    pub view: WebView,
    /// Set once the page has completed the `shell.hello` handshake;
    /// events sent earlier are coalesced until the handshake.
    ready: Cell<bool>,
    recovery: RefCell<crate::recovery::Recovery>,
    recovery_timer: RefCell<Option<glib::SourceId>>,
    queued: RefCell<Vec<Event>>,
    restorable: RefCell<Vec<Event>>,
    fallback: gtk::Box,
}

impl WebSurface {
    pub fn new(
        app: &gtk::Application,
        env: &WebEnv,
        kind: SurfaceKind,
        monitor: Option<&gdk::Monitor>,
        dispatch: Dispatch,
    ) -> Rc<Self> {
        let window = gtk::Window::builder().application(app).decorated(false).build();
        let close_animation = Rc::new(Cell::new(false));
        window.add_css_class("meridian-surface");
        // The locker is assigned to its monitor via `gtk4_session_lock`
        // instead (a different Wayland surface role; see ADR-0007). It must
        // stay unrealized until then, so skip layer-shell entirely.
        if !matches!(
            kind,
            SurfaceKind::Locker
                | SurfaceKind::Preferences
                | SurfaceKind::Finder
                | SurfaceKind::Terminal
                | SurfaceKind::Calendar
                | SurfaceKind::Clock
                | SurfaceKind::Pulse
        ) {
            configure_layer(&window, kind, monitor);
        }

        if matches!(
            kind,
            SurfaceKind::Preferences
                | SurfaceKind::Finder
                | SurfaceKind::Terminal
                | SurfaceKind::Calendar
                | SurfaceKind::Clock
                | SurfaceKind::Pulse
        ) {
            let title = match kind {
                SurfaceKind::Finder => "File Manager",
                SurfaceKind::Terminal => "Terminal",
                SurfaceKind::Calendar => "Calendar",
                SurfaceKind::Clock => "Clock",
                SurfaceKind::Pulse => "Pulse",
                _ => "Settings",
            };
            window.set_title(Some(title));
            window.set_decorated(true);
            window.add_css_class("meridian-preferences");
            if kind == SurfaceKind::Terminal {
                window.add_css_class("meridian-terminal");
            }
            let header = gtk::HeaderBar::new();
            header.set_show_title_buttons(false);
            let controls = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            controls.add_css_class("meridian-window-controls");
            for (name, class) in [("Minimize", "minimize"), ("Maximize", "maximize"), ("Close", "close")] {
                let button = gtk::Button::new();
                button.update_property(&[gtk::accessible::Property::Label(name)]);
                let glyph = gtk::DrawingArea::new();
                glyph.set_content_width(16);
                glyph.set_content_height(16);
                glyph.set_draw_func(move |glyph, cairo, _, _| {
                    let color = glyph.color();
                    cairo.set_source_rgba(
                        color.red().into(),
                        color.green().into(),
                        color.blue().into(),
                        color.alpha().into(),
                    );
                    cairo.set_line_width(1.25);
                    match class {
                        "close" => {
                            cairo.move_to(5., 5.);
                            cairo.line_to(11., 11.);
                            cairo.move_to(11., 5.);
                            cairo.line_to(5., 11.);
                        }
                        "minimize" => {
                            cairo.move_to(4.5, 8.);
                            cairo.line_to(11.5, 8.);
                        }
                        _ => {
                            cairo.rectangle(4.5, 4.5, 7., 7.);
                        }
                    }
                    let _ = cairo.stroke();
                });
                button.set_valign(gtk::Align::Center);
                button.set_halign(gtk::Align::Center);
                button.set_size_request(34, 28);
                button.set_child(Some(&glyph));
                button.set_tooltip_text(Some(name));
                button.add_css_class("meridian-window-button");
                button.add_css_class(class);
                let weak = window.downgrade();
                button.connect_clicked(move |_| {
                    if let Some(window) = weak.upgrade() {
                        match class {
                            "close" => window.close(),
                            "minimize" => window.minimize(),
                            _ if window.is_maximized() => window.unmaximize(),
                            _ => window.maximize(),
                        }
                    }
                });
                controls.append(&button);
            }
            header.pack_end(&controls);
            let title_label = gtk::Label::new(Some(title));
            title_label.set_margin_start(8);
            header.pack_start(&title_label);
            header.set_title_widget(Some(&gtk::Box::new(gtk::Orientation::Horizontal, 0)));
            window.set_titlebar(Some(&header));
            if matches!(kind, SurfaceKind::Finder | SurfaceKind::Calendar | SurfaceKind::Clock | SurfaceKind::Pulse) {
                window.set_default_size(1100, 720);
            } else {
                window.set_default_size(840, 600);
            }
            let closing = close_animation.clone();
            window.connect_close_request(move |window| {
                if closing.replace(true) {
                    return glib::Propagation::Stop;
                }
                let weak = window.downgrade();
                let started = std::time::Instant::now();
                glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                    let Some(window) = weak.upgrade() else { return glib::ControlFlow::Break };
                    let progress = (started.elapsed().as_secs_f64() / 0.16).min(1.0);
                    window.set_opacity(1.0 - progress);
                    if progress >= 1.0 {
                        window.set_visible(false);
                        window.set_opacity(1.0);
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
                glib::Propagation::Stop
            });
        }

        if crate::desktop::get().appearance == "dark" {
            window.add_css_class("meridian-dark");
        }
        let ucm = bridge::content_manager();
        let view = env.create_view(&ucm);
        let windowed = matches!(
            kind,
            SurfaceKind::Preferences
                | SurfaceKind::Finder
                | SurfaceKind::Terminal
                | SurfaceKind::Calendar
                | SurfaceKind::Clock
                | SurfaceKind::Pulse
        );
        if windowed {
            view.set_opacity(0.0);
            let animated_view = view.clone();
            let opening_close_state = close_animation.clone();
            window.connect_map(move |_| {
                opening_close_state.set(false);
                animated_view.set_opacity(0.0);
                let animated_view = animated_view.clone();
                let started = std::time::Instant::now();
                glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                    let progress = (started.elapsed().as_secs_f64() / 0.22).min(1.0);
                    animated_view.set_opacity(progress);
                    if progress >= 1.0 { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
                });
            });
        }
        if kind == SurfaceKind::Terminal {
            // Avoid stale accelerated text layers when the terminal is maximized.
            webkit6::prelude::WebViewExt::settings(&view)
                .unwrap()
                .set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Never);
            view.set_background_color(&gdk::RGBA::new(0.09, 0.10, 0.125, 1.0));
        } else {
            view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
        }
        lock_down(&view, env.dev, kind);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&view));
        let fallback = gtk::Box::new(gtk::Orientation::Vertical, 12);
        fallback.add_css_class("meridian-recovery");
        fallback.set_visible(false);
        overlay.add_overlay(&fallback);
        window.set_child(Some(&overlay));

        let surface = Rc::new(WebSurface {
            kind,
            window,
            view,
            ready: Cell::new(false),
            recovery: RefCell::default(),
            recovery_timer: RefCell::default(),
            queued: RefCell::default(),
            restorable: RefCell::default(),
            fallback,
        });
        bridge::attach(&ucm, Rc::downgrade(&surface), dispatch.clone());
        surface.install_fallback(dispatch);

        let weak = Rc::downgrade(&surface);
        surface.view.connect_web_process_terminated(move |_, reason| {
            log::error!("component=webview event=process_terminated surface={kind:?} reason={reason:?}");
            if let Some(surface) = weak.upgrade() {
                surface.recover();
            }
        });
        let weak = Rc::downgrade(&surface);
        surface.view.connect_load_failed(move |_, _, _, _| {
            if let Some(surface) = weak.upgrade() {
                surface.recover();
            }
            true
        });
        // Also recover a page that loaded but never completed its handshake.
        let weak = Rc::downgrade(&surface);
        glib::timeout_add_local_once(std::time::Duration::from_secs(20), move || {
            if let Some(surface) = weak.upgrade()
                && !surface.ready.get()
            {
                surface.recover();
            }
        });

        surface.view.load_uri(&format!("{SCHEME}://ui/{}/index.html", entry_dir(kind)));
        surface
    }

    #[cfg(any(test, feature = "integration-tests"))]
    pub fn fallback_visible(&self) -> bool {
        self.fallback.is_visible()
    }

    pub fn is_ready(&self) -> bool {
        self.ready.get()
    }

    pub fn mark_ready(&self) {
        if self.ready.replace(true) {
            return;
        }
        self.fallback.set_visible(false);
        if let Some(timer) = self.recovery_timer.borrow_mut().take() {
            timer.remove();
        }
        let mut events = self.restorable.borrow().clone();
        for event in std::mem::take(&mut *self.queued.borrow_mut()) {
            events.retain(|old| std::mem::discriminant(old) != std::mem::discriminant(&event));
            events.push(event);
        }
        for event in events {
            self.send(&event);
        }
    }

    fn recover(self: &Rc<Self>) {
        self.ready.set(false);
        self.fallback.set_visible(true);
        if self.recovery_timer.borrow().is_some() {
            return;
        }
        let delay = self.recovery.borrow_mut().failed(std::time::Instant::now());
        log::warn!("component=webview event=retry surface={:?} delay_ms={}", self.kind, delay.as_millis());
        let weak = Rc::downgrade(self);
        *self.recovery_timer.borrow_mut() = Some(glib::timeout_add_local_once(delay, move || {
            if let Some(surface) = weak.upgrade() {
                surface.recovery_timer.borrow_mut().take();
                surface.view.load_uri(&format!("{SCHEME}://ui/{}/index.html", entry_dir(surface.kind)));
                let weak = Rc::downgrade(&surface);
                glib::timeout_add_local_once(std::time::Duration::from_secs(20), move || {
                    if let Some(surface) = weak.upgrade()
                        && !surface.ready.get()
                    {
                        surface.recover();
                    }
                });
            }
        }));
    }

    fn install_fallback(self: &Rc<Self>, dispatch: Dispatch) {
        let label = gtk::Label::new(Some("Meridian is restoring this component…"));
        label.set_wrap(true);
        self.fallback.append(&label);
        if self.kind != SurfaceKind::Locker {
            return;
        }
        // A native PAM-backed unlock path stays usable if WebKit cannot restart.
        label.set_label("Session locked. Enter your password to unlock.");
        let password = gtk::PasswordEntry::new();
        let button = gtk::Button::with_label("Unlock");
        self.fallback.append(&password);
        self.fallback.append(&button);
        let weak = Rc::downgrade(self);
        let entry = password.downgrade();
        let status = label.downgrade();
        button.connect_clicked(move |button| {
            let (Some(surface), Some(entry), Some(status)) = (weak.upgrade(), entry.upgrade(), status.upgrade()) else {
                return;
            };
            let secret = entry.text().to_string();
            entry.set_text("");
            button.set_sensitive(false);
            let result = dispatch(
                &surface,
                meridian_protocol::Request::LockerUnlock(meridian_protocol::LockerUnlock { password: secret }),
            );
            let button = button.downgrade();
            glib::MainContext::default().spawn_local(async move {
                let result = match result {
                    Ok(bridge::Reply::Now(json)) => Ok(json),
                    Ok(bridge::Reply::Later(future)) => future.await,
                    Err(error) => Err(error),
                };
                if let Some(button) = button.upgrade() {
                    button.set_sensitive(true);
                }
                if let Err(error) = result {
                    status.set_label(&error.message);
                }
            });
        });
        let button = button.downgrade();
        password.connect_activate(move |_| {
            if let Some(button) = button.upgrade()
                && button.is_sensitive()
            {
                button.emit_clicked();
            }
        });
    }

    /// Push an event to the page. The serialized event is passed as a typed
    /// argument, never spliced into script source.
    pub fn send(&self, event: &Event) {
        if matches!(
            event,
            Event::MenuOpened(_) | Event::FinderLocation(_) | Event::PreferencesPage(_) | Event::PreferencesWallpaper
        ) {
            let mut saved = self.restorable.borrow_mut();
            saved.retain(|old| std::mem::discriminant(old) != std::mem::discriminant(event));
            saved.push(event.clone());
        }
        if !self.ready.get() {
            // Coalesce state by event kind, retaining commands sent during startup.
            let mut queued = self.queued.borrow_mut();
            queued.retain(|old| std::mem::discriminant(old) != std::mem::discriminant(event));
            if queued.len() < 64 {
                queued.push(event.clone());
            }
            return;
        }
        let json = match serde_json::to_string(event) {
            Ok(j) => j,
            Err(e) => return log::error!("failed to serialize {event:?}: {e}"),
        };
        let args = glib::VariantDict::new(None);
        args.insert("event", json);
        let kind = self.kind;
        self.view.call_async_javascript_function(
            "window.__meridianDispatch(event); return true;",
            Some(&args.end()),
            None,
            None,
            gtk::gio::Cancellable::NONE,
            move |result| {
                if result.is_err() {
                    log::warn!("component=webview event=dispatch_failed surface={kind:?}");
                }
            },
        );
    }
}

/// Full-width panel height; keep in sync with the UI token.
pub const PANEL_HEIGHT: i32 = 64;
pub const PANEL_ANIMATION_OVERFLOW: i32 = 20;
pub const TOPBAR_HEIGHT: i32 = 44;

fn entry_dir(kind: SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::Wallpaper => "surfaces/wallpaper",
        SurfaceKind::Panel => "surfaces/panel",
        SurfaceKind::Topbar => "surfaces/topbar",
        SurfaceKind::Applications => "surfaces/applications",
        SurfaceKind::Options => "surfaces/options",
        SurfaceKind::Notifications => "surfaces/notifications",
        SurfaceKind::Preferences => "surfaces/settings",
        SurfaceKind::Finder => "surfaces/finder",
        SurfaceKind::Terminal => "surfaces/terminal",
        SurfaceKind::Calendar | SurfaceKind::Clock => "surfaces/utilities",
        SurfaceKind::Pulse => "surfaces/pulse",
        SurfaceKind::DesktopMenu => "surfaces/desktop-menu",
        SurfaceKind::Greeter => "surfaces/greeter",
        SurfaceKind::Locker => "surfaces/locker",
    }
}

/// Configures everything except [`SurfaceKind::Locker`], which the caller
/// never passes here (see [`WebSurface::new`]).
fn configure_layer(window: &gtk::Window, kind: SurfaceKind, monitor: Option<&gdk::Monitor>) {
    window.init_layer_shell();
    window.set_monitor(monitor);
    window.set_namespace(Some(match kind {
        SurfaceKind::Wallpaper => "meridian-wallpaper",
        SurfaceKind::Panel => "meridian-panel",
        SurfaceKind::Topbar => "meridian-topbar",
        SurfaceKind::Applications => "meridian-applications",
        SurfaceKind::Options => "meridian-options",
        SurfaceKind::Notifications => "meridian-notifications",
        SurfaceKind::DesktopMenu => "meridian-desktop-menu",
        SurfaceKind::Greeter => "meridian-greeter",
        SurfaceKind::Preferences
        | SurfaceKind::Finder
        | SurfaceKind::Terminal
        | SurfaceKind::Calendar
        | SurfaceKind::Clock
        | SurfaceKind::Pulse => {
            unreachable!("preferences is a normal desktop window")
        }
        SurfaceKind::Locker => unreachable!("the locker uses gtk4_session_lock, not layer-shell"),
    }));
    let edges: &[Edge] = match kind {
        SurfaceKind::Panel => &[Edge::Bottom, Edge::Left, Edge::Right],
        SurfaceKind::Topbar => &[Edge::Top, Edge::Left, Edge::Right],
        SurfaceKind::Preferences
        | SurfaceKind::Finder
        | SurfaceKind::Terminal
        | SurfaceKind::Calendar
        | SurfaceKind::Clock
        | SurfaceKind::Pulse => {
            unreachable!("preferences is a normal desktop window")
        }
        SurfaceKind::Locker => unreachable!("the locker uses gtk4_session_lock, not layer-shell"),
        _ => &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right],
    };
    for &edge in edges {
        window.set_anchor(edge, true);
    }
    match kind {
        SurfaceKind::Wallpaper => {
            window.set_layer(Layer::Background);
            window.set_keyboard_mode(KeyboardMode::OnDemand);
            // Cover the whole output regardless of the bar.
            window.set_exclusive_zone(-1);
        }
        SurfaceKind::Panel => {
            window.set_layer(Layer::Bottom);
            window.set_keyboard_mode(KeyboardMode::None);
            window.set_default_size(1, PANEL_HEIGHT + PANEL_ANIMATION_OVERFLOW);
            window.set_exclusive_zone(PANEL_HEIGHT);
        }
        SurfaceKind::Topbar => {
            window.set_layer(Layer::Bottom);
            window.set_keyboard_mode(KeyboardMode::None);
            window.set_default_size(1, TOPBAR_HEIGHT);
            window.set_exclusive_zone(TOPBAR_HEIGHT);
        }
        SurfaceKind::Greeter => {
            // Covers everything and owns the keyboard: it's the whole screen.
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::Exclusive);
            window.set_exclusive_zone(-1);
        }
        SurfaceKind::Preferences
        | SurfaceKind::Finder
        | SurfaceKind::Terminal
        | SurfaceKind::Calendar
        | SurfaceKind::Clock
        | SurfaceKind::Pulse => {
            unreachable!("preferences is a normal desktop window")
        }
        SurfaceKind::Locker => unreachable!("the locker uses gtk4_session_lock, not layer-shell"),
        SurfaceKind::Applications | SurfaceKind::Options | SurfaceKind::DesktopMenu | SurfaceKind::Notifications => {
            window.set_layer(Layer::Overlay);
            // Exclusive zone 0: placed above the dock, which stays clickable
            // while a menu is open.
            window.set_exclusive_zone(0);
            // The Applications menu takes the keyboard so typing searches
            // immediately; Options only when clicked.
            window.set_keyboard_mode(if matches!(kind, SurfaceKind::Applications | SurfaceKind::DesktopMenu) {
                KeyboardMode::Exclusive
            } else {
                KeyboardMode::OnDemand
            });
        }
    }
}

/// Confine the WebView to `meridian://` content and deny every permission.
fn lock_down(view: &WebView, dev: bool, surface: SurfaceKind) {
    let last_external = Cell::new(None::<std::time::Instant>);
    view.connect_decide_policy(move |_, decision, kind| match kind {
        PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction => {
            let uri = decision
                .downcast_ref::<webkit6::NavigationPolicyDecision>()
                .and_then(|d| d.navigation_action())
                .and_then(|a| a.request())
                .and_then(|r| r.uri())
                .map(|u| u.to_string())
                .unwrap_or_default();
            let allowed = kind == PolicyDecisionType::NavigationAction
                && uri == format!("{SCHEME}://ui/{}/index.html", entry_dir(surface));
            if allowed {
                decision.use_();
            } else if uri == "https://github.com/saikrishna488/meridian"
                && decision
                    .downcast_ref::<webkit6::NavigationPolicyDecision>()
                    .and_then(|decision| decision.navigation_action())
                    .is_some_and(|action| action.is_user_gesture())
                && last_external.get().is_none_or(|last| last.elapsed() >= std::time::Duration::from_secs(1))
            {
                last_external.set(Some(std::time::Instant::now()));
                decision.ignore();
                let mut command = if std::env::var_os("MERIDIAN_HOST_APPS").is_some() {
                    let mut command = std::process::Command::new("flatpak-spawn");
                    command.args(["--host", "gio", "open"]);
                    command
                } else {
                    let mut command = std::process::Command::new("gio");
                    command.arg("open");
                    command
                };
                match command.arg(&uri).spawn() {
                    Ok(mut child) => {
                        std::thread::spawn(move || {
                            let _ = child.wait();
                        });
                    }
                    Err(error) => log::warn!("could not open source link: {error}"),
                }
            } else {
                log::warn!("component=webview event=navigation_blocked surface={surface:?}");
                decision.ignore();
            }
            true
        }
        // Responses: only display content; never download.
        PolicyDecisionType::Response => {
            let displayable =
                decision.downcast_ref::<webkit6::ResponsePolicyDecision>().is_some_and(|d| d.is_mime_type_supported());
            if displayable {
                decision.use_()
            } else {
                decision.ignore()
            }
            true
        }
        _ => {
            decision.ignore();
            true
        }
    });
    view.connect_permission_request(|_, request| {
        log::warn!("denied permission request {:?}", request.type_());
        request.deny();
        true
    });
    // The context menu (with "Inspect Element") only in dev mode.
    view.connect_context_menu(move |_, _, _| !dev);
}

impl Drop for WebSurface {
    fn drop(&mut self) {
        // GtkApplication also owns its windows; dropping the Rust wrapper alone
        // does not release a removed monitor's or an old lock screen's widgets.
        // gtk4-session-lock owns lock-window teardown. Destroying one here can
        // race its unlock/monitor callbacks and violates the active-lock role.
        if self.kind != SurfaceKind::Locker {
            self.window.destroy();
        }
        if let Some(timer) = self.recovery_timer.get_mut().take() {
            timer.remove();
        }
    }
}
