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
}

impl WebEnv {
    pub fn new(scheme: SchemeHandler, dev: bool) -> Self {
        let context = WebContext::new();
        let security = context.security_manager().expect("WebContext always has a security manager");
        security.register_uri_scheme_as_secure(SCHEME);
        // ES module scripts are fetched in CORS mode.
        security.register_uri_scheme_as_cors_enabled(SCHEME);
        context.register_uri_scheme(SCHEME, move |request| scheme.handle(request));

        WebEnv { context, session: NetworkSession::new_ephemeral(), first_view: RefCell::new(None), dev }
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
        configure_layer(&window, kind, monitor);

        let ucm = bridge::content_manager();
        let view = env.create_view(&ucm);
        view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
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

/// Height of the top bar in logical pixels (matches `--bar-height` in the UI).
pub const PANEL_HEIGHT: i32 = 36;

fn entry_dir(kind: SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::Wallpaper => "surfaces/wallpaper",
        SurfaceKind::Panel => "surfaces/panel",
        SurfaceKind::Applications => "surfaces/applications",
        SurfaceKind::Options => "surfaces/options",
        SurfaceKind::Greeter => "surfaces/greeter",
    }
}

fn configure_layer(window: &gtk::Window, kind: SurfaceKind, monitor: Option<&gdk::Monitor>) {
    window.init_layer_shell();
    window.set_monitor(monitor);
    window.set_namespace(Some(match kind {
        SurfaceKind::Wallpaper => "meridian-wallpaper",
        SurfaceKind::Panel => "meridian-panel",
        SurfaceKind::Applications => "meridian-applications",
        SurfaceKind::Options => "meridian-options",
        SurfaceKind::Greeter => "meridian-greeter",
    }));
    let edges: &[Edge] = match kind {
        SurfaceKind::Panel => &[Edge::Top, Edge::Left, Edge::Right],
        _ => &[Edge::Top, Edge::Bottom, Edge::Left, Edge::Right],
    };
    for &edge in edges {
        window.set_anchor(edge, true);
    }
    match kind {
        SurfaceKind::Wallpaper => {
            window.set_layer(Layer::Background);
            window.set_keyboard_mode(KeyboardMode::None);
            // Cover the whole output regardless of the bar.
            window.set_exclusive_zone(-1);
        }
        SurfaceKind::Panel => {
            window.set_layer(Layer::Top);
            window.set_keyboard_mode(KeyboardMode::None);
            window.set_default_size(1, PANEL_HEIGHT);
            // Reserve the bar's height so windows don't go under it.
            window.auto_exclusive_zone_enable();
        }
        SurfaceKind::Greeter => {
            // Covers everything and owns the keyboard: it's the whole screen.
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::Exclusive);
            window.set_exclusive_zone(-1);
        }
        SurfaceKind::Applications | SurfaceKind::Options => {
            window.set_layer(Layer::Overlay);
            // Exclusive zone 0: placed below the bar, which stays clickable
            // while a menu is open.
            window.set_exclusive_zone(0);
            // The Applications menu takes the keyboard so typing searches
            // immediately; Options only when clicked.
            window.set_keyboard_mode(if kind == SurfaceKind::Applications {
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
