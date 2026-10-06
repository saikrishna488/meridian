//! The shell: owns surfaces, services, and the request dispatcher.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use meridian_protocol::{
    self as proto, ErrorBody, ErrorCode, Event, Popover, PopoverState, Request, SurfaceInfo, SurfaceKind, WindowInfo,
};
use meridian_services::apps::{AppCatalog, HostBridge};
use meridian_services::search::apps::AppsProvider;
use meridian_services::search::{ActivationContext, SearchService};
use meridian_services::settings::SettingsService;
use meridian_services::windows::{Window, WindowService};

use crate::bridge::{Dispatch, Reply};
use crate::scheme::{Roots, SchemeHandler};
use crate::surface::{WebEnv, WebSurface};

/// A fixed shell action (keybinding).
type ShellAction = fn(&Shell);

/// How long a menu may take to animate out before the host hides it anyway
/// (guards against a stuck or crashed page).
const DISMISS_TIMEOUT: Duration = Duration::from_millis(500);

const POPOVERS: [Popover; 2] = [Popover::Applications, Popover::Options];

pub struct Shell {
    app: gtk::Application,
    env: WebEnv,
    catalog: Rc<RefCell<AppCatalog>>,
    search: SearchService,
    settings: SettingsService,
    windows: Option<WindowService>,
    dev: bool,
    state: RefCell<State>,
    weak_self: Weak<Shell>,
    _app_monitor: gio::AppInfoMonitor,
    _hold: gio::ApplicationHoldGuard,
}

#[derive(Default)]
struct State {
    primary: Option<gdk::Monitor>,
    panel: Option<Rc<WebSurface>>,
    wallpapers: Vec<(gdk::Monitor, Rc<WebSurface>)>,
    popovers: HashMap<Popover, Rc<WebSurface>>,
    dismiss_timers: HashMap<Popover, glib::SourceId>,
}

impl Shell {
    pub fn start(app: &gtk::Application, roots: Roots, dev: bool) -> Rc<Shell> {
        let host = HostBridge::from_env();
        let catalog = Rc::new(RefCell::new(AppCatalog::load(host.clone())));

        let mut search = SearchService::new();
        search.register(Box::new(AppsProvider::new(catalog.clone())));

        let windows = match WindowService::connect() {
            Ok(w) => Some(w),
            Err(e) => {
                log::warn!("running-window list disabled: {e}");
                None
            }
        };

        let app_monitor = gio::AppInfoMonitor::get();
        let shell = Rc::new_cyclic(|weak_self| Shell {
            app: app.clone(),
            env: WebEnv::new(SchemeHandler::new(roots), dev),
            catalog,
            search,
            settings: SettingsService::new(),
            windows,
            dev,
            state: RefCell::new(State::default()),
            weak_self: weak_self.clone(),
            _app_monitor: app_monitor.clone(),
            _hold: app.hold(),
        });

        // Installed/removed applications.
        let weak = Rc::downgrade(&shell);
        app_monitor.connect_changed(move |_| {
            if let Some(shell) = weak.upgrade() {
                *shell.catalog.borrow_mut() = AppCatalog::load(host.clone());
                shell.send_to_popover(Popover::Applications, &Event::SearchInvalidated);
            }
        });

        let weak = Rc::downgrade(&shell);
        shell.settings.connect_changed(move |state| {
            if let Some(shell) = weak.upgrade() {
                shell.send_to_popover(Popover::Options, &Event::SettingsChanged(state.clone()));
            }
        });

        if let Some(windows) = &shell.windows {
            let weak = Rc::downgrade(&shell);
            windows.connect_changed(move |list| {
                if let Some(shell) = weak.upgrade() {
                    let infos = shell.window_infos(list);
                    shell.send_panel(&Event::WindowsChanged(infos));
                }
            });
        }

        // Menus are created hidden so they open instantly.
        for popover in POPOVERS {
            let surface = WebSurface::new(app, &shell.env, popover_surface(popover), None, shell.dispatcher());
            shell.state.borrow_mut().popovers.insert(popover, surface);
        }

        let monitors = gdk::Display::default().expect("GTK is initialized").monitors();
        let weak = Rc::downgrade(&shell);
        monitors.connect_items_changed(move |_, _, _, _| {
            if let Some(shell) = weak.upgrade() {
                shell.reconcile_monitors();
            }
        });
        shell.reconcile_monitors();
        shell.install_actions();
        log::info!("meridian shell started{}", if dev { " (dev mode)" } else { "" });
        shell
    }

    fn dispatcher(&self) -> Dispatch {
        let weak = self.weak_self.clone();
        Rc::new(move |surface, request| match weak.upgrade() {
            Some(shell) => shell.dispatch(surface, request).map(Reply::Now),
            None => Err(ErrorBody::new(ErrorCode::Unavailable, "shell is shutting down")),
        })
    }

    /// Execute an authorized request (capabilities are checked by the bridge).
    fn dispatch(&self, surface: &Rc<WebSurface>, request: Request) -> Result<String, ErrorBody> {
        let reply = match request {
            Request::Hello => {
                surface.mark_ready();
                proto::reply_ok(&SurfaceInfo {
                    surface: surface.kind,
                    capabilities: surface.kind.capabilities().to_vec(),
                    dev: self.dev,
                })
            }
            Request::SearchQuery(q) => proto::reply_ok(&self.search.query(&q.query, q.serial)),
            Request::SearchActivate(a) => {
                let launch_context = gdk::Display::default().map(|d| d.app_launch_context());
                let ctx = ActivationContext { launch_context: launch_context.as_ref().map(|c| c.upcast_ref()) };
                self.search.activate(a.provider, &a.item_id, &ctx)?;
                ok()
            }
            Request::PopoverToggle(t) => {
                self.toggle_popover(t.popover);
                ok()
            }
            Request::PopoverClose => {
                let popover = POPOVERS
                    .into_iter()
                    .find(|&p| popover_surface(p) == surface.kind)
                    .ok_or_else(|| ErrorBody::new(ErrorCode::InvalidRequest, "not a menu"))?;
                self.hide_popover(popover);
                ok()
            }
            Request::WindowsList => {
                let list = self.windows.as_ref().map(WindowService::windows).unwrap_or_default();
                proto::reply_ok(&self.window_infos(&list))
            }
            Request::WindowsActivate(a) => {
                let windows = self.windows.as_ref().ok_or_else(|| {
                    ErrorBody::new(ErrorCode::Unavailable, "the compositor does not share its window list")
                })?;
                if !windows.activate(a.id) {
                    return Err(ErrorBody::new(ErrorCode::NotFound, "no such window"));
                }
                ok()
            }
            Request::SettingsGet => proto::reply_ok(&self.settings.state()),
            Request::SettingsSetWifi(v) => {
                self.settings.set_wifi(v.enabled)?;
                ok()
            }
            Request::SettingsSetBluetooth(v) => {
                self.settings.set_bluetooth(v.enabled)?;
                ok()
            }
            Request::SettingsSetBrightness(v) => {
                self.settings.set_brightness(v.percent)?;
                ok()
            }
            Request::SettingsSetVolume(v) => {
                self.settings.set_volume(v.percent)?;
                ok()
            }
            // Capability-gated to the greeter surface, which this shell never creates.
            Request::GreeterInfo | Request::GreeterLogin(_) | Request::GreeterPower(_) => {
                return Err(ErrorBody::new(ErrorCode::InvalidRequest, "not a login screen"));
            }
        };
        reply.map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("serializing reply: {e}")))
    }

    fn window_infos(&self, windows: &[Window]) -> Vec<WindowInfo> {
        let catalog = self.catalog.borrow();
        windows
            .iter()
            .map(|w| WindowInfo {
                id: w.id,
                name: catalog
                    .name_for_window(&w.app_id)
                    .map(str::to_owned)
                    .or_else(|| (!w.app_id.is_empty()).then(|| w.app_id.clone()))
                    .unwrap_or_else(|| w.title.clone()),
                title: w.title.clone(),
                active: w.active,
            })
            .collect()
    }

    fn send_panel(&self, event: &Event) {
        let panel = self.state.borrow().panel.clone();
        if let Some(panel) = panel {
            panel.send(event);
        }
    }

    fn send_to_popover(&self, popover: Popover, event: &Event) {
        let surface = self.state.borrow().popovers.get(&popover).cloned();
        if let Some(surface) = surface {
            surface.send(event);
        }
    }

    // ---- menus -----------------------------------------------------------

    /// Visible and not on its way out.
    fn is_open(&self, popover: Popover) -> bool {
        let state = self.state.borrow();
        state.popovers.get(&popover).is_some_and(|s| s.window.is_visible())
            && !state.dismiss_timers.contains_key(&popover)
    }

    fn toggle_popover(&self, popover: Popover) {
        log::debug!("toggle {popover:?} (open: {})", self.is_open(popover));
        if self.is_open(popover) {
            self.dismiss_popover(popover);
        } else {
            self.open_popover(popover);
        }
    }

    fn open_popover(&self, popover: Popover) {
        let Some(surface) = self.state.borrow().popovers.get(&popover).cloned() else { return };
        for other in POPOVERS.into_iter().filter(|&p| p != popover) {
            self.dismiss_popover(other);
        }
        self.cancel_dismiss_timer(popover);
        // M0: menus open on the primary output, where the bar is.
        let primary = self.state.borrow().primary.clone();
        gtk4_layer_shell::LayerShell::set_monitor(&surface.window, primary.as_ref());
        surface.window.present();
        surface.send(&Event::PopoverShown);
        if popover == Popover::Options {
            self.settings.refresh();
        }
        self.send_popover_state();
    }

    /// Ask the menu to animate out; it replies with `popover.close`.
    fn dismiss_popover(&self, popover: Popover) {
        if !self.is_open(popover) {
            return;
        }
        self.send_to_popover(popover, &Event::PopoverDismiss);
        let weak = self.weak_self.clone();
        let timer = glib::timeout_add_local_once(DISMISS_TIMEOUT, move || {
            if let Some(shell) = weak.upgrade() {
                shell.state.borrow_mut().dismiss_timers.remove(&popover);
                log::debug!("{popover:?} did not close itself in time; hiding");
                shell.hide_popover(popover);
            }
        });
        self.state.borrow_mut().dismiss_timers.insert(popover, timer);
        self.send_popover_state();
    }

    fn hide_popover(&self, popover: Popover) {
        self.cancel_dismiss_timer(popover);
        let surface = self.state.borrow().popovers.get(&popover).cloned();
        if let Some(surface) = surface {
            surface.window.set_visible(false);
        }
        self.send_popover_state();
    }

    fn cancel_dismiss_timer(&self, popover: Popover) {
        let timer = self.state.borrow_mut().dismiss_timers.remove(&popover);
        if let Some(timer) = timer {
            timer.remove();
        }
    }

    fn send_popover_state(&self) {
        let state =
            PopoverState { applications: self.is_open(Popover::Applications), options: self.is_open(Popover::Options) };
        self.send_panel(&Event::PopoverState(state));
    }

    // ---- outputs ---------------------------------------------------------

    /// Create/destroy per-output surfaces to match the current outputs:
    /// every output gets a wallpaper; the primary output also gets the bar.
    fn reconcile_monitors(&self) {
        let display = gdk::Display::default().expect("GTK is initialized");
        let monitors: Vec<gdk::Monitor> = display.monitors().iter::<gdk::Monitor>().filter_map(Result::ok).collect();
        let primary = monitors.first().cloned();
        log::info!("outputs: {}", monitors.iter().map(describe).collect::<Vec<_>>().join(", "));

        if self.state.borrow().primary != primary {
            let old_panel = {
                let mut state = self.state.borrow_mut();
                state.primary = primary.clone();
                state.panel.take()
            };
            if let Some(panel) = old_panel {
                panel.window.destroy();
            }
            if let Some(monitor) = &primary {
                let panel = WebSurface::new(&self.app, &self.env, SurfaceKind::Panel, Some(monitor), self.dispatcher());
                panel.window.present();
                self.state.borrow_mut().panel = Some(panel);
            }
        }

        let removed: Vec<Rc<WebSurface>> = {
            let mut state = self.state.borrow_mut();
            let (keep, removed): (Vec<_>, Vec<_>) =
                std::mem::take(&mut state.wallpapers).into_iter().partition(|(m, _)| monitors.contains(m));
            state.wallpapers = keep;
            removed.into_iter().map(|(_, s)| s).collect()
        };
        removed.iter().for_each(|s| s.window.destroy());
        for monitor in &monitors {
            if self.state.borrow().wallpapers.iter().any(|(m, _)| m == monitor) {
                continue;
            }
            let wallpaper =
                WebSurface::new(&self.app, &self.env, SurfaceKind::Wallpaper, Some(monitor), self.dispatcher());
            wallpaper.window.present();
            self.state.borrow_mut().wallpapers.push((monitor.clone(), wallpaper));
        }
    }

    // ---- actions ---------------------------------------------------------

    /// Fixed, argument-less actions exported on D-Bus for compositor
    /// keybindings: `gapplication action org.meridian.Shell <name>`.
    fn install_actions(&self) {
        let actions: [(&str, ShellAction); 2] = [
            ("toggle-applications", |shell| shell.toggle_popover(Popover::Applications)),
            ("toggle-options", |shell| shell.toggle_popover(Popover::Options)),
        ];
        for (name, handler) in actions {
            let action = gio::SimpleAction::new(name, None);
            let weak = self.weak_self.clone();
            action.connect_activate(move |_, _| {
                if let Some(shell) = weak.upgrade() {
                    handler(&shell);
                }
            });
            self.app.add_action(&action);
        }
    }
}

fn popover_surface(popover: Popover) -> SurfaceKind {
    match popover {
        Popover::Applications => SurfaceKind::Applications,
        Popover::Options => SurfaceKind::Options,
    }
}

fn ok() -> Result<String, serde_json::Error> {
    proto::reply_ok(&())
}

fn describe(m: &gdk::Monitor) -> String {
    let g = m.geometry();
    // Refresh rate is reported in mHz; 0 means unknown (e.g. nested outputs).
    let refresh = match m.refresh_rate() {
        0 => "?".to_owned(),
        mhz => format!("{:.0}", mhz as f64 / 1000.0),
    };
    format!(
        "{} {}x{}@{:.2}x {refresh}Hz",
        m.connector().map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
        g.width(),
        g.height(),
        m.scale(),
    )
}
