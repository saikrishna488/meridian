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

/// Web engine state shared by all surfaces: one context (one URI scheme
/// handler), one ephemeral network session, one web process.
pub struct WebEnv {
    context: WebContext,
    session: NetworkSession,
    /// First WebView created; later ones are related to it so they share
    /// its web process.
    first_view: RefCell<Option<glib::WeakRef<WebView>>>,
    dev: bool,
    scheme: Rc<SchemeHandler>,
}

impl WebEnv {
    pub fn new(scheme: SchemeHandler, dev: bool) -> Self {
        let scheme = Rc::new(scheme);
        let context = WebContext::new();
        let security = context.security_manager().expect("WebContext always has a security manager");
        security.register_uri_scheme_as_secure(SCHEME);
        // ES module scripts are fetched in CORS mode.
        security.register_uri_scheme_as_cors_enabled(SCHEME);
        let handler = scheme.clone();
        context.register_uri_scheme(SCHEME, move |request| handler.handle(request));

        WebEnv { context, session: NetworkSession::new_ephemeral(), first_view: RefCell::new(None), dev, scheme }
    }

    pub fn check_locker(&self) -> Result<(), String> {
        self.scheme.check_locker()
    }

    fn settings(&self) -> webkit6::Settings {
        let s = webkit6::Settings::new();
        s.set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Always);
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
        let related = self.first_view.borrow().as_ref().and_then(|w| w.upgrade());
        let builder = WebView::builder().user_content_manager(ucm).settings(&self.settings());
        let view = match related {
            // A related view inherits context and session, and shares the
            // web process.
            Some(first) => builder.related_view(&first).build(),
            None => builder.web_context(&self.context).network_session(&self.session).build(),
        };
        if self.first_view.borrow().is_none() {
            *self.first_view.borrow_mut() = Some(view.downgrade());
        }
        view
    }
}

pub struct WebSurface {
    pub kind: SurfaceKind,
    pub window: gtk::Window,
    pub view: WebView,
    /// Set once the page has completed the `shell.hello` handshake;
    /// events sent earlier are dropped (pages fetch state on load).
    ready: Cell<bool>,
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
        window.add_css_class("meridian-surface");
        // The locker is assigned to its monitor via `gtk4_session_lock`
        // instead (a different Wayland surface role; see ADR-0007). It must
        // stay unrealized until then, so skip layer-shell entirely.
        if !matches!(kind, SurfaceKind::Locker | SurfaceKind::Preferences | SurfaceKind::Finder | SurfaceKind::Terminal)
        {
            configure_layer(&window, kind, monitor);
        }

        if matches!(kind, SurfaceKind::Preferences | SurfaceKind::Finder | SurfaceKind::Terminal) {
            let title = match kind {
                SurfaceKind::Finder => "Finder",
                SurfaceKind::Terminal => "Terminal",
                _ => "Settings",
            };
            window.set_title(Some(&format!("Meridian {title}")));
            window.set_decorated(true);
            window.add_css_class("meridian-preferences");
            if kind == SurfaceKind::Terminal {
                window.add_css_class("meridian-terminal");
            }
            let header = gtk::HeaderBar::new();
            header.set_show_title_buttons(false);
            let controls = gtk::Box::new(gtk::Orientation::Horizontal, 7);
            controls.add_css_class("meridian-window-controls");
            for (name, class) in [("Close", "close"), ("Minimize", "minimize"), ("Maximize", "maximize")] {
                let button = gtk::Button::new();
                button.update_property(&[gtk::accessible::Property::Label(name)]);
                let glyph = gtk::DrawingArea::new();
                glyph.set_content_width(16);
                glyph.set_content_height(16);
                glyph.set_draw_func(move |_, cairo, _, _| {
                    cairo.set_source_rgba(0., 0., 0., 0.65);
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
                            cairo.move_to(5., 11.);
                            cairo.line_to(11., 5.);
                            cairo.move_to(6., 5.);
                            cairo.line_to(11., 5.);
                            cairo.line_to(11., 10.);
                        }
                    }
                    let _ = cairo.stroke();
                });
                button.set_valign(gtk::Align::Center);
                button.set_halign(gtk::Align::Center);
                button.set_size_request(18, 18);
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
            header.pack_start(&controls);
            header.set_title_widget(Some(&gtk::Label::new(Some(title))));
            window.set_titlebar(Some(&header));
            if kind == SurfaceKind::Finder {
                window.set_default_size(1100, 720);
            } else {
                window.set_default_size(840, 600);
            }
            window.connect_close_request(|window| {
                window.set_visible(false);
                glib::Propagation::Stop
            });
        }

        if crate::desktop::get().appearance == "dark" {
            window.add_css_class("meridian-dark");
        }
        let ucm = bridge::content_manager();
        let view = env.create_view(&ucm);
        if kind == SurfaceKind::Terminal {
            // Avoid stale accelerated text layers when the terminal is maximized.
            webkit6::prelude::WebViewExt::settings(&view)
                .unwrap()
                .set_hardware_acceleration_policy(webkit6::HardwareAccelerationPolicy::Never);
            view.set_background_color(&gdk::RGBA::new(0.09, 0.10, 0.125, 1.0));
        } else {
            view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
        }
        lock_down(&view, env.dev);
        window.set_child(Some(&view));

        let surface = Rc::new(WebSurface { kind, window, view, ready: Cell::new(false) });
        bridge::attach(&ucm, Rc::downgrade(&surface), dispatch);

        // Recover from web process crashes instead of leaving a dead surface.
        let weak = Rc::downgrade(&surface);
        surface.view.connect_web_process_terminated(move |view, reason| {
            log::error!("web process for {kind:?} terminated ({reason:?}); reloading");
            if let Some(s) = weak.upgrade() {
                s.ready.set(false);
            }
            view.reload();
        });

        surface.view.load_uri(&format!("{SCHEME}://ui/{}/index.html", entry_dir(kind)));
        surface
    }

    pub fn mark_ready(&self) {
        self.ready.set(true);
    }

    /// Push an event to the page. The serialized event is passed as a typed
    /// argument, never spliced into script source.
    pub fn send(&self, event: &Event) {
        if !self.ready.get() {
            log::debug!("{:?} not ready; dropping {event:?}", self.kind);
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
            "window.__meridianDispatch(event);",
            Some(&args.end()),
            None,
            None,
            gtk::gio::Cancellable::NONE,
            move |result| {
                if let Err(e) = result {
                    log::warn!("dispatching event to {kind:?} failed: {e}");
                }
            },
        );
    }
}

/// Dock surface height, including room for magnification and tooltips.
pub const PANEL_HEIGHT: i32 = 128;

fn entry_dir(kind: SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::Wallpaper => "surfaces/wallpaper",
        SurfaceKind::Panel | SurfaceKind::Topbar => "surfaces/panel",
        SurfaceKind::Applications => "surfaces/applications",
        SurfaceKind::Options => "surfaces/options",
        SurfaceKind::Preferences => "surfaces/settings",
        SurfaceKind::Finder => "surfaces/finder",
        SurfaceKind::Terminal => "surfaces/terminal",
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
        SurfaceKind::DesktopMenu => "meridian-desktop-menu",
        SurfaceKind::Greeter => "meridian-greeter",
        SurfaceKind::Preferences | SurfaceKind::Finder | SurfaceKind::Terminal => {
            unreachable!("preferences is a normal desktop window")
        }
        SurfaceKind::Locker => unreachable!("the locker uses gtk4_session_lock, not layer-shell"),
    }));
    let edges: &[Edge] = match kind {
        SurfaceKind::Panel => &[Edge::Bottom],
        SurfaceKind::Topbar => &[Edge::Top, Edge::Left, Edge::Right],
        SurfaceKind::Preferences | SurfaceKind::Finder | SurfaceKind::Terminal => {
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
            window.set_layer(Layer::Top);
            window.set_keyboard_mode(KeyboardMode::None);
            let width = monitor.map_or(720, |m| m.geometry().width().min(720));
            window.set_default_size(width, PANEL_HEIGHT);
            // Reserve only the visible dock (64px) and bottom gap (6px).
            // The rest of the surface is headroom for hover motion/tooltips.
            window.set_exclusive_zone(70);
        }
        SurfaceKind::Topbar => {
            window.set_layer(Layer::Top);
            window.set_keyboard_mode(KeyboardMode::None);
            window.set_default_size(1, 32);
            window.set_exclusive_zone(32);
        }
        SurfaceKind::Greeter => {
            // Covers everything and owns the keyboard: it's the whole screen.
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::Exclusive);
            window.set_exclusive_zone(-1);
        }
        SurfaceKind::Preferences | SurfaceKind::Finder | SurfaceKind::Terminal => {
            unreachable!("preferences is a normal desktop window")
        }
        SurfaceKind::Locker => unreachable!("the locker uses gtk4_session_lock, not layer-shell"),
        SurfaceKind::Applications | SurfaceKind::Options | SurfaceKind::DesktopMenu => {
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
fn lock_down(view: &WebView, dev: bool) {
    view.connect_decide_policy(|_, decision, kind| match kind {
        PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction => {
            let uri = decision
                .downcast_ref::<webkit6::NavigationPolicyDecision>()
                .and_then(|d| d.navigation_action())
                .and_then(|a| a.request())
                .and_then(|r| r.uri())
                .map(|u| u.to_string())
                .unwrap_or_default();
            let allowed = kind == PolicyDecisionType::NavigationAction && uri.starts_with(&format!("{SCHEME}://ui/"));
            if allowed {
                decision.use_();
            } else if uri == "https://github.com/saikrishna488/meridian" {
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
                log::warn!("blocked navigation to {uri:?}");
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
