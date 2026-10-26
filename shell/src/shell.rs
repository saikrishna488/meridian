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

mod settings;
mod surfaces;
mod terminal;

/// A fixed shell action (keybinding).
type ShellAction = fn(&Shell);

/// How long a menu may take to animate out before the host hides it anyway
/// (guards against a stuck or crashed page).
const DISMISS_TIMEOUT: Duration = Duration::from_millis(500);

const POPOVERS: [Popover; 4] = [Popover::Applications, Popover::Options, Popover::DesktopMenu, Popover::Notifications];

pub struct Shell {
    app: gtk::Application,
    env: WebEnv,
    catalog: Rc<RefCell<AppCatalog>>,
    search: SearchService,
    search_preferences: RefCell<proto::SearchPreferences>,
    settings: SettingsService,
    windows: RefCell<Option<WindowService>>,
    /// The logged-in user's lock screen checks their password against.
    username: String,
    locker: gtk4_session_lock::Instance,
    terminals: RefCell<crate::terminal::Terminals>,
    transfers: RefCell<crate::transfer::Transfers>,
    pulse: std::sync::Arc<std::sync::Mutex<crate::pulse::Sampler>>,
    notifications: RefCell<Option<Rc<crate::notifications::Notifications>>>,
    dev: bool,
    state: RefCell<State>,
    weak_self: Weak<Shell>,
    _app_monitor: gio::AppInfoMonitor,
    _hold: gio::ApplicationHoldGuard,
}

#[derive(Default)]
struct State {
    primary: Option<gdk::Monitor>,
    desktop_monitor: Option<gio::FileMonitor>,
    desktop_refresh_timer: Option<glib::SourceId>,
    catalog_refresh_timer: Option<glib::SourceId>,
    panel: Option<Rc<WebSurface>>,
    topbar: Option<Rc<WebSurface>>,
    preferences: Option<Rc<WebSurface>>,
    finder: Option<Rc<WebSurface>>,
    finder_monitor: Option<gio::FileMonitor>,
    finder_monitor_path: Option<String>,
    finder_monitor_generation: u64,
    finder_refresh_timer: Option<glib::SourceId>,
    terminal: Option<Rc<WebSurface>>,
    pulse: Option<Rc<WebSurface>>,
    calendar: Option<Rc<WebSurface>>,
    clock: Option<Rc<WebSurface>>,
    wallpapers: Vec<(gdk::Monitor, Rc<WebSurface>)>,
    popovers: HashMap<Popover, Rc<WebSurface>>,
    dismiss_timers: HashMap<Popover, glib::SourceId>,
    /// One lock surface per monitor, live only while locked (ADR-0007).
    lock_surfaces: Vec<(gdk::Monitor, Rc<WebSurface>)>,
}

impl Shell {
    pub fn start(app: &gtk::Application, roots: Roots, dev: bool) -> Rc<Shell> {
        let desktop = crate::desktop::get();
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(true);
            settings.set_gtk_font_name(Some(&format!("{} 11", desktop.font)));
        }
        glib::spawn_future_local(async {
            if let Err(error) = crate::files::blocking(crate::desktop::apply_fonts).await {
                log::warn!("Desktop font defaults could not be applied: {error:?}");
            }
        });
        glib::spawn_future_local(async {
            if let Err(error) = crate::files::blocking(crate::settings_devices::apply_saved_displays).await {
                log::warn!("Saved displays could not be applied: {error:?}");
            }
        });
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
            env: WebEnv::new(SchemeHandler::new(roots).with_catalog(catalog.clone()), dev),
            catalog,
            search,
            search_preferences: RefCell::new(crate::search::preferences()),
            settings: SettingsService::new(),
            windows: RefCell::new(windows),
            username: glib::user_name().to_string_lossy().into_owned(),
            locker: gtk4_session_lock::Instance::new(),
            terminals: RefCell::new(crate::terminal::Terminals::default()),
            transfers: RefCell::default(),
            pulse: Default::default(),
            notifications: RefCell::new(None),
            dev,
            state: RefCell::new(State::default()),
            weak_self: weak_self.clone(),
            _app_monitor: app_monitor.clone(),
            _hold: app.hold(),
        });

        let weak = Rc::downgrade(&shell);
        *shell.notifications.borrow_mut() = Some(crate::notifications::Notifications::start(move |state| {
            if let Some(shell) = weak.upgrade() {
                let event = Event::NotificationsChanged(state);
                shell.send_panel(&event);
                shell.send_to_popover(Popover::Notifications, &event);
                shell.send_to_popover(Popover::Options, &event);
            }
        }));

        // The session-lock protocol (ADR-0007): a lock surface per monitor,
        // torn down once the lock screen accepts a password.
        let weak = Rc::downgrade(&shell);
        shell.locker.connect_monitor(move |instance, monitor| {
            if let Some(shell) = weak.upgrade() {
                shell.add_lock_surface(instance, monitor);
            }
        });
        let weak = Rc::downgrade(&shell);
        shell.locker.connect_unlocked(move |_| {
            if let Some(shell) = weak.upgrade() {
                let surfaces = std::mem::take(&mut shell.state.borrow_mut().lock_surfaces);
                drop(surfaces); // gtk4-session-lock destroys its assigned windows.
            }
        });
        let weak = Rc::downgrade(&shell);
        shell.locker.connect_failed(move |_| {
            log::error!("could not lock the session (unsupported compositor, or already locked elsewhere)");
            if let Some(shell) = weak.upgrade() {
                let surfaces = std::mem::take(&mut shell.state.borrow_mut().lock_surfaces);
                drop(surfaces); // gtk4-session-lock owns failed-lock teardown too.
            }
        });

        // Installed/removed applications.
        let weak = Rc::downgrade(&shell);
        app_monitor.connect_changed(move |_| {
            if let Some(shell) = weak.upgrade() {
                if let Some(timer) = shell.state.borrow_mut().catalog_refresh_timer.take() {
                    timer.remove();
                }
                let weak = shell.weak_self.clone();
                let host = host.clone();
                let timer = glib::timeout_add_local_once(Duration::from_millis(250), move || {
                    if let Some(shell) = weak.upgrade() {
                        shell.state.borrow_mut().catalog_refresh_timer.take();
                        *shell.catalog.borrow_mut() = AppCatalog::load(host);
                        shell.env.invalidate_icons();
                        shell.send_app_catalog_invalidated();
                    }
                });
                shell.state.borrow_mut().catalog_refresh_timer = Some(timer);
            }
        });

        let weak = Rc::downgrade(&shell);
        shell.settings.connect_changed(move |state| {
            if let Some(shell) = weak.upgrade() {
                shell.send_to_popover(Popover::Options, &Event::SettingsChanged(state.clone()));
                if let Some(surface) = &shell.state.borrow().preferences {
                    surface.send(&Event::SettingsChanged(state.clone()));
                }
            }
        });

        shell.watch_windows();
        let weak = shell.weak_self.clone();
        glib::timeout_add_local(Duration::from_secs(5), move || {
            let Some(shell) = weak.upgrade() else { return glib::ControlFlow::Break };
            if shell.windows.borrow().as_ref().is_none_or(|service| !service.is_available()) {
                let service = WindowService::connect().ok();
                *shell.windows.borrow_mut() = service;
                shell.watch_windows();
                let list = shell.windows.borrow().as_ref().map(WindowService::windows).unwrap_or_default();
                shell.update_dock_visibility(&list);
                shell.send_panel(&Event::WindowsChanged(shell.window_infos(&list)));
            }
            glib::ControlFlow::Continue
        });

        let monitors = gdk::Display::default().expect("GTK is initialized").monitors();
        let weak = Rc::downgrade(&shell);
        monitors.connect_items_changed(move |_, _, _, _| {
            if let Some(shell) = weak.upgrade() {
                shell.reconcile_monitors();
            }
        });
        let desktop_folder = crate::desktop::folder();
        if let Err(error) = std::fs::create_dir_all(&desktop_folder) {
            log::warn!("Desktop folder: {error}");
        }
        if let Ok(monitor) = gio::File::for_path(desktop_folder)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            let weak = Rc::downgrade(&shell);
            monitor.connect_changed(move |_, _, _, _| {
                if let Some(shell) = weak.upgrade() {
                    if let Some(timer) = shell.state.borrow_mut().desktop_refresh_timer.take() {
                        timer.remove();
                    }
                    let weak = shell.weak_self.clone();
                    let timer = glib::timeout_add_local_once(Duration::from_millis(150), move || {
                        if let Some(shell) = weak.upgrade() {
                            shell.state.borrow_mut().desktop_refresh_timer.take();
                            shell.desktop_changed(&crate::desktop::get());
                        }
                    });
                    shell.state.borrow_mut().desktop_refresh_timer = Some(timer);
                }
            });
            shell.state.borrow_mut().desktop_monitor = Some(monitor);
        }
        shell.reconcile_monitors();
        shell.install_actions();
        // Critical desktop surfaces get the first rendering budget. Warm the two
        // frequent menus afterward; applications and occasional menus stay lazy.
        let weak = shell.weak_self.clone();
        glib::timeout_add_local_once(Duration::from_secs(1), move || {
            if let Some(shell) = weak.upgrade() {
                shell.ensure_popover(Popover::Applications);
                shell.ensure_popover(Popover::Options);
            }
        });
        log::info!("meridian shell started{}", if dev { " (dev mode)" } else { "" });
        shell
    }

    pub fn shutdown(&self) {
        self.transfers.borrow().cancel_all();
        self.terminals.borrow_mut().clear();
        for window in self.app.windows() {
            if !self.state.borrow().lock_surfaces.iter().any(|(_, surface)| surface.window == window) {
                window.destroy();
            }
        }
    }

    fn dispatcher(&self) -> Dispatch {
        let weak = self.weak_self.clone();
        Rc::new(move |surface, request| match weak.upgrade() {
            Some(shell) => shell.dispatch(surface, request),
            None => Err(ErrorBody::new(ErrorCode::Unavailable, "shell is shutting down")),
        })
    }

    /// Execute an authorized request (capabilities are checked by the bridge).
    fn dispatch(&self, surface: &Rc<WebSurface>, request: Request) -> Result<Reply, ErrorBody> {
        if self.locker.is_locked()
            && !matches!(request, Request::Hello | Request::DesktopGet | Request::LockerUnlock(_))
        {
            return Err(ErrorBody::new(ErrorCode::PermissionDenied, "The session is locked."));
        }
        if matches!(
            request,
            Request::SessionSignOut | Request::SessionRestart | Request::SessionShutdown | Request::SessionSleep
        ) && self.transfers.borrow().has_active()
        {
            return Err(ErrorBody::new(
                ErrorCode::Unavailable,
                "A file transfer is running. Wait for it or cancel it before ending or suspending the session.",
            ));
        }
        match request {
            Request::Hello => {
                let first = !surface.is_ready();
                if first && surface.kind == SurfaceKind::Terminal {
                    self.terminals.borrow_mut().clear();
                }
                surface.mark_ready();
                if first
                    && surface.window.is_visible()
                    && POPOVERS.into_iter().any(|popover| popover_surface(popover) == surface.kind)
                {
                    surface.send(&Event::PopoverShown);
                }
                self.send_popover_state();
                if surface.kind == SurfaceKind::Pulse {
                    surface.send(&Event::PulseVisibility(surface.window.is_visible()));
                }
                surface.send(&Event::DesktopChanged(crate::desktop::get()));
                if surface.kind == SurfaceKind::Terminal && surface.window.is_visible() {
                    surface.send(&Event::TerminalOpened);
                }
                serialize(proto::reply_ok(&SurfaceInfo {
                    surface: surface.kind,
                    capabilities: surface.kind.capabilities().to_vec(),
                    dev: self.dev,
                }))
            }
            Request::DesktopFontSet(choice) => {
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let state = crate::files::blocking(move || crate::desktop::set_font(&choice.id)).await?;
                    if let Some(settings) = gtk::Settings::default() {
                        settings.set_gtk_font_name(Some(&format!("{} 11", state.font)));
                    }
                    if let Some(shell) = weak.upgrade() {
                        shell.desktop_changed(&state);
                    }
                    proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::DesktopAppearanceSet(choice) => {
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let state = crate::files::blocking(move || crate::desktop::set_appearance(&choice.id)).await?;
                    if let Some(shell) = weak.upgrade() {
                        if let Some(settings) = gtk::Settings::default() {
                        settings.set_gtk_application_prefer_dark_theme(true);
                        }
                        shell.desktop_changed(&state);
                    }
                    proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::DesktopGet => serialize(proto::reply_ok(&crate::desktop::get())),
            Request::DesktopWallpaperSet(choice) => {
                let state = crate::desktop::set_wallpaper(&choice.id)?;
                self.desktop_changed(&state);
                serialize(proto::reply_ok(&state))
            }
            Request::DesktopWallpaperChoose => {
                let window = surface.window.clone();
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let filter = gtk::FileFilter::new();
                    filter.set_name(Some("Wallpaper images"));
                    for mime in ["image/png", "image/jpeg", "image/webp"] {
                        filter.add_mime_type(mime);
                    }
                    let filters = gio::ListStore::new::<gtk::FileFilter>();
                    filters.append(&filter);
                    let dialog = gtk::FileDialog::builder().title("Choose wallpaper").filters(&filters).build();
                    let file = match dialog.open_future(Some(&window)).await {
                        Ok(file) => file,
                        Err(error) if error.matches(gtk::DialogError::Dismissed) => {
                            return proto::reply_ok(&crate::desktop::get())
                                .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()));
                        }
                        Err(error) => return Err(ErrorBody::new(ErrorCode::Failed, error.to_string())),
                    };
                    let path = file
                        .path()
                        .ok_or_else(|| ErrorBody::new(ErrorCode::InvalidRequest, "Choose a local image."))?;
                    let state = crate::files::blocking(move || crate::desktop::choose_wallpaper(&path)).await?;
                    if let Some(shell) = weak.upgrade() {
                        shell.desktop_changed(&state);
                    }
                    proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::DesktopAddApp(app) => {
                let (name, icon) = if let Some(found) = self.catalog.borrow().get(&app.id) {
                    (found.name.clone(), found.icon.clone())
                } else {
                    let name = match app.id.as_str() {
                        "meridian-settings.desktop" => "Settings",
                        "meridian-finder.desktop" => "File Manager",
                        "meridian-terminal.desktop" => "Meridian Terminal",
                        "meridian-pulse.desktop" => "Pulse",
                        _ => return Err(ErrorBody::new(ErrorCode::NotFound, "Unknown application")),
                    };
                    (name.into(), None)
                };
                let state = crate::desktop::add(proto::DesktopItem {
                    id: format!("app:{}", app.id),
                    name,
                    icon,
                    kind: "app".into(),
                    target: app.id,
                })?;
                self.desktop_changed(&state);
                serialize(proto::reply_ok(&state))
            }
            Request::DesktopAddFile(file) => {
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let state = crate::files::blocking(move || crate::desktop::add_file(&file.path)).await?;
                    if let Some(shell) = weak.upgrade() {
                        shell.desktop_changed(&state);
                    }
                    proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::DesktopChooseFiles => {
                let window = surface.window.clone();
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let dialog = gtk::FileDialog::builder().title("Move files to Desktop").build();
                    let files = match dialog.open_multiple_future(Some(&window)).await {
                        Ok(files) => files,
                        Err(error) if error.matches(gtk::DialogError::Dismissed) => {
                            return proto::reply_ok(&crate::desktop::get())
                                .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()));
                        }
                        Err(error) => return Err(ErrorBody::new(ErrorCode::Failed, error.to_string())),
                    };
                    let mut state = crate::desktop::get();
                    for index in 0..files.n_items() {
                        if let Some(file) = files.item(index).and_downcast::<gio::File>().and_then(|file| file.path()) {
                            state = crate::files::blocking(move || crate::desktop::add_file(&file.to_string_lossy()))
                                .await?;
                        }
                    }
                    if let Some(shell) = weak.upgrade() {
                        shell.desktop_changed(&state);
                    }
                    proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::DesktopRemove(app) => {
                let state = crate::desktop::remove(&app.id)?;
                self.desktop_changed(&state);
                serialize(proto::reply_ok(&state))
            }
            Request::DesktopOpen(app) => {
                let item = crate::desktop::item(&app.id)?;
                if item.kind != "app" {
                    if std::path::Path::new(&item.target).is_dir() {
                        crate::files::open_directory(&item.target)?;
                    } else {
                        crate::files::open(&item.target)?;
                    }
                } else {
                    match item.target.as_str() {
                        "meridian-settings.desktop" => self.open_preferences(),
                        "meridian-finder.desktop" => self.open_finder(),
                        "meridian-terminal.desktop" => self.open_terminal(),
                        "meridian-calendar.desktop" => self.catalog.borrow().launch("org.gnome.Calendar.desktop", None)
                            .map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("{e:?}")))?,
                        "meridian-clock.desktop" => self.catalog.borrow().launch("org.gnome.clocks.desktop", None)
                            .map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("{e:?}")))?,
                        "meridian-pulse.desktop" => self.open_pulse(),
                        id => self
                            .catalog
                            .borrow()
                            .launch(id, None)
                            .map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("{e:?}")))?,
                    }
                }
                serialize(ok())
            }
            Request::MenuOpen(menu) => {
                if ![
                    "Meridian",
                    "File",
                    "Edit",
                    "View",
                    "Window",
                    "Help",
                    "Application",
                    "DockItem",
                ]
                .contains(&menu.menu.as_str()) {
                    return Err(ErrorBody::new(ErrorCode::InvalidRequest, "unknown desktop menu"));
                }
                self.open_popover(Popover::DesktopMenu);
                self.send_to_popover(Popover::DesktopMenu, &Event::MenuOpened(menu));
                serialize(ok())
            }
            Request::NotificationsGet
            | Request::NotificationsRead
            | Request::NotificationsDoNotDisturb(_)
            | Request::NotificationsClear
            | Request::NotificationsDismiss(_)
            | Request::NotificationsAction(_) => {
                let service = self
                    .notifications
                    .borrow()
                    .clone()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "Notifications unavailable"))?;
                match request {
                    Request::NotificationsRead => service.mark_read(),
                    Request::NotificationsDoNotDisturb(choice) => service.set_do_not_disturb(choice.enabled)?,
                    Request::NotificationsClear => service.clear(),
                    Request::NotificationsDismiss(item) => service.dismiss(item.id),
                    Request::NotificationsAction(action) => service.activate(action.id, &action.key),
                    _ => {}
                }
                serialize(proto::reply_ok(&service.snapshot()))
            }
            Request::CalendarList => serialize(proto::reply_ok(&crate::calendar::list()?)),
            Request::CalendarSave(events) => {
                crate::calendar::save(&events)?;
                serialize(ok())
            }
            Request::PulseOpen => {
                self.open_pulse();
                serialize(ok())
            }
            Request::PulseSnapshot => {
                let sampler = self.pulse.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let snapshot = crate::files::blocking(move || {
                        sampler
                            .lock()
                            .map_err(|_| ErrorBody::new(ErrorCode::Failed, "Pulse sampler unavailable"))?
                            .sample()
                    })
                    .await?;
                    proto::reply_ok(&snapshot).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::TerminalOpen
            | Request::TerminalOpenAt(_)
            | Request::TerminalStart
            | Request::TerminalStartAt(_)
            | Request::TerminalCopy(_)
            | Request::TerminalPaste
            | Request::TerminalRead(_)
            | Request::TerminalWrite(_)
            | Request::TerminalResize(_)
            | Request::TerminalClose(_) => self.dispatch_terminal(request),
            Request::FinderOpen => {
                self.open_finder();
                serialize(ok())
            }
            Request::FileBookmarks => serialize(proto::reply_ok(&crate::files::bookmarks())),
            Request::FileBookmarkSet(file) => Ok(Reply::Later(Box::pin(async move {
                let bookmarks = crate::files::blocking(move || crate::files::bookmark(&file.path)).await?;
                proto::reply_ok(&bookmarks).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileChooseFolder => {
                let parent = surface.window.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let dialog = gtk::FileDialog::builder().title("Choose a destination").modal(true).build();
                    let path = match dialog.select_folder_future(Some(&parent)).await {
                        Ok(file) => file.path().map(|p| p.to_string_lossy().into_owned()),
                        Err(error) if error.matches(gtk::DialogError::Dismissed) => None,
                        Err(error) => return Err(ErrorBody::new(ErrorCode::Failed, error.to_string())),
                    };
                    proto::reply_ok(&path).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::FileLocations => serialize(proto::reply_ok(&crate::files::locations())),
            Request::FileList(list) => {
                let generation = {
                    let mut state = self.state.borrow_mut();
                    state.finder_monitor_generation = state.finder_monitor_generation.wrapping_add(1);
                    state.finder_monitor_generation
                };
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let result = crate::files::blocking(move || crate::files::list(&list.path, list.hidden)).await?;
                    if let Some(shell) = weak.upgrade() {
                        shell.watch_finder(&result.path, generation);
                    }
                    proto::reply_ok(&result).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::FileProperties(file) => Ok(Reply::Later(Box::pin(async move {
                let result = crate::files::blocking(move || crate::files::properties(&file.path)).await?;
                proto::reply_ok(&result).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileTrashList => Ok(Reply::Later(Box::pin(async move {
                let listing = crate::files::blocking(crate::trash::list).await?;
                proto::reply_ok(&listing).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileTrashRestore(item) => Ok(Reply::Later(Box::pin(async move {
                let path = crate::files::blocking(move || crate::trash::restore(&item.uri)).await?;
                proto::reply_ok(&path).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileOpen(file) => {
                crate::files::open(&file.path)?;
                serialize(ok())
            }
            Request::FileMkdir(file) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::files::mkdir(&file.path, &file.name)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileRename(file) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::files::rename(&file.path, &file.name)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileTransfers => serialize(proto::reply_ok(&self.transfers.borrow().list())),
            Request::FileTransferStart(choice) => {
                serialize(proto::reply_ok(&self.transfers.borrow_mut().start(choice)?))
            }
            Request::FileTransferStatus(job) => serialize(proto::reply_ok(&self.transfers.borrow().status(job.id)?)),
            Request::FileTransferCancel(job) => {
                self.transfers.borrow().cancel(job.id)?;
                serialize(ok())
            }
            Request::FileTransfer(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || {
                    crate::files::transfer(&choice.path, &choice.destination, choice.move_file)
                })
                .await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FilePreview(file) => Ok(Reply::Later(Box::pin(async move {
                let preview = crate::files::blocking(move || crate::files::preview(&file.path)).await?;
                proto::reply_ok(&preview).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileThumbnail(file) => Ok(Reply::Later(Box::pin(async move {
                let thumbnail = crate::files::blocking(move || crate::files::thumbnail(&file.path)).await?;
                proto::reply_ok(&thumbnail).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::FileWallpaper(file) => {
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let state = crate::files::blocking(move || {
                        crate::desktop::choose_wallpaper(std::path::Path::new(&file.path))
                    })
                    .await?;
                    if let Some(shell) = weak.upgrade() {
                        shell.desktop_changed(&state);
                    }
                    proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::FileTrash(file) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::files::trash(&file.path)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::PreferencesOpen => {
                self.open_preferences();
                serialize(ok())
            }
            Request::PreferencesWallpaperOpen => {
                self.open_preferences();
                if let Some(surface) = &self.state.borrow().preferences {
                    surface.send(&Event::PreferencesWallpaper);
                }
                serialize(ok())
            }
            Request::PreferencesPageOpen(page) => {
                self.open_preferences();
                if let Some(surface) = &self.state.borrow().preferences {
                    surface.send(&Event::PreferencesPage(page));
                }
                serialize(ok())
            }
            Request::TrashOpen => {
                crate::files::open_trash()?;
                serialize(ok())
            }
            Request::AppUninstallPrompt(app) => {
                if self.catalog.borrow().get(&app.id).is_none() {
                    return Err(ErrorBody::new(ErrorCode::NotFound, "This application cannot be uninstalled"));
                }
                self.open_popover(Popover::Applications);
                self.send_to_popover(Popover::Applications, &Event::AppUninstallRequested(app));
                serialize(ok())
            }
            Request::AppUninstallPlan(app) => {
                let app = self
                    .catalog
                    .borrow()
                    .get(&app.id)
                    .cloned()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::NotFound, "No such installed application"))?;
                Ok(Reply::Later(Box::pin(async move {
                    let plan = crate::uninstall::plan(&app).await?;
                    proto::reply_ok(&plan).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::AppUninstall(request) => {
                let app = self
                    .catalog
                    .borrow()
                    .get(&request.id)
                    .cloned()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::NotFound, "No such installed application"))?;
                let catalog = self.catalog.clone();
                let host = HostBridge::from_env();
                let recipients: Vec<_> = self
                    .state
                    .borrow()
                    .popovers
                    .values()
                    .cloned()
                    .chain(self.state.borrow().panel.iter().cloned())
                    .collect();
                Ok(Reply::Later(Box::pin(async move {
                    crate::uninstall::uninstall(&app, &request.target).await?;
                    *catalog.borrow_mut() = AppCatalog::load(host);
                    for surface in &recipients {
                        surface.send(&Event::SearchInvalidated);
                    }
                    let mut pins = read_dock();
                    pins.retain(|id| id != &request.id);
                    for surface in &recipients {
                        surface.send(&Event::DockChanged(pins.clone()));
                    }
                    let path = dock_path();
                    if let Ok(bytes) = serde_json::to_vec(&pins) {
                        let _ = std::fs::write(path, bytes);
                    }
                    proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SystemInfo => serialize(proto::reply_ok(&system_info())),
            Request::DockLayoutGet => serialize(proto::reply_ok(&read_dock_layout())),
            Request::DockLayoutSet(layout) => {
                let unique: std::collections::HashSet<_> = layout.items.iter().collect();
                if layout.items.len() > 512
                    || unique.len() != layout.items.len()
                    || layout.items.iter().any(|id| id.len() > 512 || id.is_empty())
                {
                    return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Invalid dock layout"));
                }
                let path = dock_path().with_file_name("dock-layout.json");
                std::fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| std::fs::write(&path, serde_json::to_vec(&layout.items).unwrap()))
                    .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?;
                self.send_panel(&Event::DockLayoutChanged(layout.items.clone()));
                self.send_to_popover(Popover::Applications, &Event::DockLayoutChanged(layout.items));
                serialize(ok())
            }
            Request::DockList => serialize(proto::reply_ok(&read_dock())),
            Request::DockPin(pin) => {
                if pin.pinned
                    && ![
                        "meridian-settings.desktop",
                        "meridian-pulse.desktop",
                    ]
                    .contains(&pin.id.as_str())
                    && !self.catalog.borrow().apps().iter().any(|app| app.id == pin.id)
                {
                    return Err(ErrorBody::new(ErrorCode::NotFound, "no such application"));
                }
                let mut pins = read_dock();
                pins.retain(|id| id != &pin.id);
                if pin.pinned {
                    pins.push(pin.id);
                }
                let path = dock_path();
                std::fs::create_dir_all(path.parent().unwrap())
                    .and_then(|_| std::fs::write(&path, serde_json::to_vec(&pins).unwrap()))
                    .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?;
                self.send_panel(&Event::DockChanged(pins.clone()));
                self.send_to_popover(Popover::Applications, &Event::DockChanged(pins));
                serialize(ok())
            }
            Request::SearchQuery(q) => {
                let mut results = self.search.query(&q.query, q.serial);
                results.sections.extend(crate::search::sections(&q.query, &self.search_preferences.borrow()));
                let query = q.query.trim().to_lowercase();
                let builtins = [
                    (
                        "meridian-pulse.desktop",
                        "Pulse",
                        "Meridian pulse task manager system monitor processes cpu gpu memory storage network",
                        "utilities-system-monitor",
                    ),
                    ("meridian-settings.desktop", "Settings", "Meridian settings preferences", "preferences-system"),
                ];
                let items = builtins
                    .into_iter()
                    .filter_map(|(id, name, keywords, icon)| {
                        let tokens: Vec<_> = query.split_whitespace().map(meridian_services::search::fuzzy::fold_token).collect();
                        let candidate = meridian_services::search::fuzzy::Prepared::new(&format!("{name} {keywords}"));
                        tokens.iter().all(|token| meridian_services::search::fuzzy::fuzzy(token, &candidate).is_some()).then_some((id, name, icon))
                    })
                    .map(|(id, name, icon)| proto::SearchItem {
                        id: id.into(),
                        title: name.into(),
                        subtitle: Some(if name.starts_with("Meridian ") {
                            "Interactive terminal".into()
                        } else {
                            format!("Meridian {name}")
                        }),
                        icon: Some(icon.into()),
                    })
                    .collect::<Vec<_>>();
                if !items.is_empty() {
                    results.sections.push(proto::SearchSection {
                        provider: proto::ProviderId::Apps,
                        title: "Meridian".into(),
                        items,
                    });
                }
                serialize(proto::reply_ok(&results))
            }
            Request::SearchActivate(a) => {
                if a.provider == proto::ProviderId::Settings {
                    if !self.search_preferences.borrow().settings {
                        return Err(ErrorBody::new(ErrorCode::PermissionDenied, "Settings search is disabled"));
                    }
                    let page = crate::search::page(&a.item_id)
                        .ok_or_else(|| ErrorBody::new(ErrorCode::NotFound, "No such settings page"))?;
                    self.open_preferences();
                    if let Some(surface) = &self.state.borrow().preferences {
                        surface.send(&Event::PreferencesPage(proto::PreferencesPageRequest { page }));
                    }
                    return serialize(ok());
                }
                if a.provider == proto::ProviderId::Calculator {
                    if !self.search_preferences.borrow().calculator {
                        return Err(ErrorBody::new(ErrorCode::PermissionDenied, "Calculator search is disabled"));
                    }
                    let result = crate::search::calculate(&a.item_id)
                        .ok_or_else(|| ErrorBody::new(ErrorCode::InvalidRequest, "Invalid calculation"))?;
                    gdk::Display::default()
                        .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "No clipboard"))?
                        .clipboard()
                        .set_text(&result);
                    return serialize(ok());
                }
                if a.item_id == "meridian-calendar.desktop" {
                    self.catalog.borrow().launch("org.gnome.Calendar.desktop", None)
                        .map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("{e:?}")))?;
                    return serialize(ok());
                }
                if a.item_id == "meridian-clock.desktop" {
                    self.catalog.borrow().launch("org.gnome.clocks.desktop", None)
                        .map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("{e:?}")))?;
                    return serialize(ok());
                }
                if a.item_id == "meridian-pulse.desktop" {
                    self.open_pulse();
                    return serialize(ok());
                }
                if a.item_id == "meridian-terminal.desktop" {
                    self.open_terminal();
                    return serialize(ok());
                }
                if a.item_id == "meridian-finder.desktop" {
                    self.open_finder();
                    return serialize(ok());
                }
                if a.item_id == "meridian-settings.desktop" {
                    self.open_preferences();
                    return serialize(ok());
                }
                let launch_context = gdk::Display::default().map(|d| d.app_launch_context());
                let ctx = ActivationContext { launch_context: launch_context.as_ref().map(|c| c.upcast_ref()) };
                self.search.activate(a.provider, &a.item_id, &ctx)?;
                serialize(ok())
            }
            Request::PopoverToggle(t) => {
                self.toggle_popover(t.popover);
                serialize(ok())
            }
            Request::PopoverClose => {
                let popover = POPOVERS
                    .into_iter()
                    .find(|&p| popover_surface(p) == surface.kind)
                    .ok_or_else(|| ErrorBody::new(ErrorCode::InvalidRequest, "not a menu"))?;
                self.hide_popover(popover);
                serialize(ok())
            }
            Request::WindowsList => {
                let list = self.windows.borrow().as_ref().map(WindowService::windows).unwrap_or_default();
                serialize(proto::reply_ok(&self.window_infos(&list)))
            }
            Request::WindowsActivate(a) => {
                let windows = self.windows.borrow().clone().ok_or_else(|| {
                    ErrorBody::new(ErrorCode::Unavailable, "the compositor does not share its window list")
                })?;
                if !windows.activate(a.id) {
                    return Err(ErrorBody::new(ErrorCode::NotFound, "no such window"));
                }
                serialize(ok())
            }
            Request::WindowsClose(a) => {
                let windows = self.windows.borrow().clone().ok_or_else(|| {
                    ErrorBody::new(ErrorCode::Unavailable, "the compositor does not share its window list")
                })?;
                if !windows.close(a.id) {
                    return Err(ErrorBody::new(ErrorCode::NotFound, "no such window"));
                }
                serialize(ok())
            }
            Request::SettingsPrimaryDisplay
            | Request::SettingsPrimaryDisplaySet(_)
            | Request::SettingsSearch
            | Request::SettingsSearchSet(_)
            | Request::SettingsDisplays
            | Request::SettingsDisplayApply(_)
            | Request::SettingsDisplayDecide(_)
            | Request::SettingsMonitorBrightness(_)
            | Request::SettingsWifiNetworks
            | Request::SettingsWifiProfiles
            | Request::SettingsWifiConfig(_)
            | Request::SettingsWifiConfigSet(_)
            | Request::SettingsWifiProfileAction(_)
            | Request::SettingsWifiConnect(_)
            | Request::SettingsBluetoothDevices
            | Request::SettingsBluetoothScan
            | Request::SettingsBluetoothAction(_)
            | Request::SettingsUsers
            | Request::SettingsUserCreate(_)
            | Request::SettingsUserAction(_)
            | Request::SettingsStartupApps
            | Request::SettingsStartup
            | Request::SettingsStartupSet(_)
            | Request::SettingsStartupAdd(_)
            | Request::SettingsSystemInfo
            | Request::SettingsCheckUpdates
            | Request::SettingsOpenReleases
            | Request::SettingsDevices
            | Request::SettingsDeviceAction(_)
            | Request::SettingsDrivers
            | Request::SettingsDriverSet(_)
            | Request::PowerStatus
            | Request::SettingsIdle
            | Request::SettingsIdleSet(_)
            | Request::SettingsAudio
            | Request::SettingsAudioAction(_)
            | Request::SettingsGet
            | Request::SettingsSetWifi(_)
            | Request::SettingsSetBluetooth(_)
            | Request::SettingsSetBrightness(_)
            | Request::SettingsSetVolume(_) => self.dispatch_settings(surface, request),
            Request::SessionLock => {
                self.lock_session()?;
                serialize(ok())
            }
            Request::SessionSignOut => Ok(Reply::Later(Box::pin(async move {
                crate::power::sign_out().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SessionSleep => {
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let shell =
                        weak.upgrade().ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "Session ended"))?;
                    shell.lock_session_confirmed().await?;
                    crate::power::suspend().await?;
                    proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SessionRestart => Ok(Reply::Later(Box::pin(async move {
                crate::power::reboot().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SessionShutdown => Ok(Reply::Later(Box::pin(async move {
                crate::power::power_off().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::LockerUnlock(u) => {
                if !self.accepts_unlock(surface) {
                    return Err(ErrorBody::new(ErrorCode::PermissionDenied, "This lock screen is no longer active."));
                }
                let lock_surface = Rc::downgrade(surface);
                let weak = self.weak_self.clone();
                let username = self.username.clone();
                Ok(Reply::Later(Box::pin(async move {
                    crate::lock_auth::verify(username, u.password).await?;
                    if let (Some(shell), Some(surface)) = (weak.upgrade(), lock_surface.upgrade())
                        && shell.accepts_unlock(&surface)
                    {
                        shell.locker.unlock();
                    }
                    proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            // Capability-gated to the greeter surface, which this shell never creates.
            Request::GreeterInfo | Request::GreeterLogin(_) | Request::GreeterPower(_) => {
                Err(ErrorBody::new(ErrorCode::InvalidRequest, "not a login screen"))
            }
        }
    }

    // ---- lock screen (ADR-0007) ------------------------------------------

    /// Start a lock (idempotent); `connect_monitor` creates the per-monitor
    /// surfaces. Closing the Options menu first avoids resuming into a
    /// dropdown that has nothing left open behind it.
    fn lock_session(&self) -> Result<(), ErrorBody> {
        let child = std::process::Command::new("swaylock")
            .args(["--daemonize", "--color", "000000"])
            .spawn()
            .map_err(|e| ErrorBody::new(ErrorCode::Unavailable, format!("Could not start swaylock: {e}")))?;
        std::thread::spawn(move || {
            let mut child = child;
            let _ = child.wait();
        });
        for popover in POPOVERS {
            self.dismiss_popover(popover);
        }
        Ok(())
    }

    async fn lock_session_confirmed(&self) -> Result<(), ErrorBody> {
        self.lock_session()?;
        Ok(())
    }

    fn accepts_unlock(&self, surface: &Rc<WebSurface>) -> bool {
        self.locker.is_locked()
            && self.state.borrow().lock_surfaces.iter().any(|(_, current)| Rc::ptr_eq(current, surface))
    }

    fn add_lock_surface(&self, instance: &gtk4_session_lock::Instance, monitor: &gdk::Monitor) {
        if self.state.borrow().lock_surfaces.iter().any(|(existing, _)| existing == monitor) {
            return;
        }
        let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Locker, Some(monitor), self.dispatcher());
        instance.assign_window_to_monitor(&surface.window, monitor);
        self.state.borrow_mut().lock_surfaces.push((monitor.clone(), surface));
    }

    fn desktop_changed(&self, state: &proto::DesktopState) {
        let event = Event::DesktopChanged(state.clone());
        let surfaces = self.state.borrow();
        for (_, surface) in &surfaces.wallpapers {
            surface.send(&event);
        }
        for (_, surface) in &surfaces.lock_surfaces {
            surface.send(&event);
        }
        for surface in [
            &surfaces.preferences,
            &surfaces.finder,
            &surfaces.terminal,
            &surfaces.pulse,
            &surfaces.calendar,
            &surfaces.clock,
            &surfaces.panel,
            &surfaces.topbar,
        ]
        .into_iter()
        .flatten()
        {
            if state.appearance == "dark" {
                surface.window.add_css_class("meridian-dark");
            } else {
                surface.window.remove_css_class("meridian-dark");
            }
            surface.send(&event);
        }
        for surface in surfaces.popovers.values() {
            surface.send(&event);
        }
    }

    fn watch_finder(&self, path: &str, generation: u64) {
        let state = self.state.borrow();
        if state.finder_monitor_generation != generation || state.finder_monitor_path.as_deref() == Some(path) {
            return;
        }
        drop(state);
        let monitor = match gio::File::for_path(path)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            Ok(monitor) => monitor,
            Err(e) => {
                log::debug!("Folder monitoring unavailable: {e}");
                return;
            }
        };
        let weak = self.weak_self.clone();
        let watched = path.to_owned();
        monitor.connect_changed(move |_, _, _, _| {
            let Some(shell) = weak.upgrade() else { return };
            let mut state = shell.state.borrow_mut();
            if state.finder_monitor_path.as_deref() != Some(&watched) {
                return;
            }
            if let Some(timer) = state.finder_refresh_timer.take() {
                timer.remove();
            }
            let weak = shell.weak_self.clone();
            let path = watched.clone();
            state.finder_refresh_timer = Some(glib::timeout_add_local_once(Duration::from_millis(150), move || {
                if let Some(shell) = weak.upgrade() {
                    let mut state = shell.state.borrow_mut();
                    state.finder_refresh_timer.take();
                    if state.finder_monitor_path.as_deref() == Some(&path)
                        && let Some(surface) = &state.finder
                    {
                        surface.send(&Event::FilesChanged(proto::FilePath { path }));
                    }
                }
            }));
        });
        let mut state = self.state.borrow_mut();
        if let Some(timer) = state.finder_refresh_timer.take() {
            timer.remove();
        }
        if let Some(old) = state.finder_monitor.take() {
            old.cancel();
        }
        state.finder_monitor_path = Some(path.to_owned());
        state.finder_monitor = Some(monitor);
    }

    fn watch_windows(&self) {
        if let Some(windows) = &*self.windows.borrow() {
            let weak = self.weak_self.clone();
            windows.connect_changed(move |list| {
                if let Some(shell) = weak.upgrade() {
                    shell.update_dock_visibility(list);
                    let infos = shell.window_infos(list);
                    shell.send_to_popover(Popover::Applications, &Event::WindowsChanged(infos.clone()));
                    shell.send_panel(&Event::WindowsChanged(infos));
                }
            });
        }
    }

    fn update_dock_visibility(&self, windows: &[Window]) {
        let state = self.state.borrow();
        for surface in [&state.panel, &state.topbar].into_iter().flatten() {
            surface.window.set_visible(!windows.iter().any(|window| window.fullscreen));
        }
    }

    fn window_infos(&self, windows: &[Window]) -> Vec<WindowInfo> {
        let catalog = self.catalog.borrow();
        windows
            .iter()
            .map(|w| WindowInfo {
                id: w.id,
                name: if matches!(w.title.as_str(), "Terminal" | "Meridian Terminal") {
                    "Meridian Terminal".into()
                } else if matches!(w.title.as_str(), "Orbit" | "File Manager" | "Meridian Finder") {
                    "File Manager".into()
                } else if matches!(w.title.as_str(), "Settings" | "Meridian Settings") {
                    "Settings".into()
                } else {
                    catalog
                        .name_for_window(&w.app_id)
                        .map(str::to_owned)
                        .or_else(|| (!w.app_id.is_empty()).then(|| w.app_id.clone()))
                        .unwrap_or_else(|| w.title.clone())
                },
                title: w.title.clone(),
                active: w.active,
            })
            .collect()
    }

    fn send_app_catalog_invalidated(&self) {
        let surfaces = {
            let state = self.state.borrow();
            state
                .popovers
                .values()
                .cloned()
                .chain(state.panel.iter().cloned())
                .chain(state.topbar.iter().cloned())
                .chain(state.preferences.iter().cloned())
                .chain(state.finder.iter().cloned())
                .chain(state.wallpapers.iter().map(|(_, surface)| surface.clone()))
                .collect::<Vec<_>>()
        };
        for surface in surfaces {
            surface.send(&Event::SearchInvalidated);
        }
    }

    fn send_panel(&self, event: &Event) {
        let topbar = self.state.borrow().topbar.clone();
        if let Some(topbar) = topbar {
            topbar.send(event);
        }
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
        let surface = self.ensure_popover(popover);
        for other in POPOVERS.into_iter().filter(|&p| p != popover) {
            // Menus share the overlay layer. Hide the existing overlay before
            // mapping the desktop menu so a full-screen Applications or
            // Control Center surface cannot remain above its context menu.
            if popover == Popover::DesktopMenu {
                self.hide_popover(other);
            } else {
                self.dismiss_popover(other);
            }
        }
        self.cancel_dismiss_timer(popover);
        // M0: menus open on the primary output, where the bar is.
        let primary = self.state.borrow().primary.clone();
        gtk4_layer_shell::LayerShell::set_monitor(&surface.window, primary.as_ref());
        surface.window.present();
        surface.send(&Event::PopoverShown);
        if popover == Popover::Notifications
            && let Some(service) = self.notifications.borrow().as_ref()
        {
            service.mark_read();
        }
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
        let state = PopoverState {
            applications: self.is_open(Popover::Applications),
            options: self.is_open(Popover::Options),
            notifications: self.is_open(Popover::Notifications),
        };
        self.send_panel(&Event::PopoverState(state));
        self.update_dock_visibility(
            &self.windows.borrow().as_ref().map(|service| service.windows()).unwrap_or_default(),
        );
    }

    // ---- outputs ---------------------------------------------------------

    /// Create/destroy per-output surfaces to match the current outputs:
    /// every output gets a wallpaper; the primary output also gets the bar.
    fn reconcile_monitors(&self) {
        let display = gdk::Display::default().expect("GTK is initialized");
        let monitors: Vec<gdk::Monitor> = display.monitors().iter::<gdk::Monitor>().filter_map(Result::ok).collect();
        let removed_locks: Vec<_> = {
            let mut state = self.state.borrow_mut();
            let (keep, removed): (Vec<_>, Vec<_>) = std::mem::take(&mut state.lock_surfaces)
                .into_iter()
                .partition(|(monitor, _)| monitors.contains(monitor));
            state.lock_surfaces = keep;
            removed.into_iter().map(|(_, surface)| surface).collect()
        };
        // The lock library unmaps/destroys windows when their monitors vanish.
        drop(removed_locks);
        let preferred = crate::desktop::primary_display();
        let primary = monitors
            .iter()
            .find(|monitor| monitor.connector().as_deref() == preferred.as_deref() && preferred.is_some())
            .cloned()
            .or_else(|| monitors.first().cloned());
        log::info!("outputs: {}", monitors.iter().map(describe).collect::<Vec<_>>().join(", "));

        if self.state.borrow().primary != primary {
            for popover in POPOVERS {
                self.hide_popover(popover);
            }
            let old_panel = {
                let mut state = self.state.borrow_mut();
                state.primary = primary.clone();
                if let Some(topbar) = state.topbar.take() {
                    topbar.window.destroy();
                }
                state.panel.take()
            };
            if let Some(panel) = old_panel {
                panel.window.destroy();
            }
            if let Some(monitor) = &primary {
                let panel = WebSurface::new(&self.app, &self.env, SurfaceKind::Panel, Some(monitor), self.dispatcher());
                let topbar =
                    WebSurface::new(&self.app, &self.env, SurfaceKind::Topbar, Some(monitor), self.dispatcher());
                topbar.window.present();
                self.state.borrow_mut().topbar = Some(topbar);
                panel.window.present();
                self.state.borrow_mut().panel = Some(panel);
                if let Some(windows) = &*self.windows.borrow() {
                    self.update_dock_visibility(&windows.windows());
                }
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
            let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::MOVE);
            drop.set_propagation_phase(gtk::PropagationPhase::Capture);
            let weak = self.weak_self.clone();
            drop.connect_drop(move |_, value, _, _| {
                let Ok(files) = value.get::<gdk::FileList>() else {
                    return false;
                };
                let Some(shell) = weak.upgrade() else {
                    return false;
                };
                let paths: Vec<_> = files.files().into_iter().filter_map(|file| file.path()).collect();
                if paths.is_empty() {
                    return false;
                }
                let weak = shell.weak_self.clone();
                glib::spawn_future_local(async move {
                    for path in paths {
                        let result =
                            crate::files::blocking(move || crate::desktop::add_file(&path.to_string_lossy())).await;
                        if let Some(shell) = weak.upgrade() {
                            match result {
                                Ok(state) => shell.desktop_changed(&state),
                                Err(error) => {
                                    log::warn!("component=desktop event=drop_failed code={:?}", error.code);
                                    let notification = gio::Notification::new("Could not move a file to Desktop");
                                    notification.set_body(Some(&error.message));
                                    shell.app.send_notification(Some("desktop-drop-failed"), &notification);
                                }
                            }
                        }
                    }
                });
                true
            });
            wallpaper.window.add_controller(drop);
            wallpaper.window.present();
            self.state.borrow_mut().wallpapers.push((monitor.clone(), wallpaper));
        }
    }

    // ---- actions ---------------------------------------------------------

    /// Fixed, argument-less actions exported on D-Bus for compositor
    /// keybindings: `gapplication action org.meridian.Shell <name>`.
    fn take_screenshot(&self, mode: &'static str) {
        if self.locker.is_locked() {
            return;
        }
        std::thread::spawn(move || {
            let result = std::process::Command::new("sh")
                .args(["-c", include_str!("../../session/screenshot.sh"), "meridian-screenshot", mode])
                .status();
            if let Err(error) = result {
                log::warn!("Could not start screenshot: {error}");
            }
        });
    }

    fn install_actions(&self) {
        let actions: [(&str, ShellAction); 8] = [
            ("toggle-applications", |shell| shell.toggle_popover(Popover::Applications)),
            ("toggle-options", |shell| shell.toggle_popover(Popover::Options)),
            ("open-finder", Shell::open_finder),
            ("open-terminal", Shell::open_terminal),
            ("open-settings", Shell::open_preferences),
            ("screenshot", |shell| shell.take_screenshot("full")),
            ("screenshot-region", |shell| shell.take_screenshot("region")),
            ("lock-session", |shell| {
                if let Err(e) = shell.lock_session() {
                    log::warn!("lock-session keybinding: {}", e.message);
                }
            }),
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
        Popover::Notifications => SurfaceKind::Notifications,
        Popover::DesktopMenu => SurfaceKind::DesktopMenu,
    }
}

fn serialize(result: Result<String, serde_json::Error>) -> Result<Reply, ErrorBody> {
    result.map(Reply::Now).map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("serializing reply: {e}")))
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

fn dock_path() -> std::path::PathBuf {
    glib::user_config_dir().join("meridian/dock.json")
}

fn read_dock() -> Vec<String> {
    std::fs::read(dock_path())
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_else(|| vec!["meridian-settings.desktop".into()])
}

fn system_info() -> proto::SystemInfo {
    let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default().trim().to_string();
    let field = |text: &str, key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap_or("Unavailable")
            .trim()
            .trim_matches('"')
            .to_string()
    };
    let os = read("/etc/os-release");
    let cpu = read("/proc/cpuinfo");
    let mem = read("/proc/meminfo");
    let memory = field(&mem, "MemTotal:")
        .split_whitespace()
        .next()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|kb| format!("{:.1} GiB", kb / 1048576.0))
        .unwrap_or_else(|| "Unavailable".into());
    proto::SystemInfo {
        kernel: read("/proc/sys/kernel/osrelease"),
        architecture: std::env::consts::ARCH.into(),
        hostname: read("/proc/sys/kernel/hostname"),
        version: env!("CARGO_PKG_VERSION").into(),
        manufacturer: read("/sys/class/dmi/id/sys_vendor"),
        model: read("/sys/class/dmi/id/product_name"),
        os: field(&os, "PRETTY_NAME="),
        processor: field(&cpu, "model name\t:"),
        memory,
    }
}

fn read_dock_layout() -> Vec<String> {
    std::fs::read(dock_path().with_file_name("dock-layout.json"))
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

#[cfg(any(test, feature = "integration-tests"))]
#[path = "shell_tests.rs"]
mod tests;

#[cfg(feature = "integration-tests")]
pub fn run_nested_smoke() {
    tests::nested_wayland_smoke();
}
