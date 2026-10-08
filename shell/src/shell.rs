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

const POPOVERS: [Popover; 3] = [Popover::Applications, Popover::Options, Popover::DesktopMenu];

pub struct Shell {
    app: gtk::Application,
    env: WebEnv,
    catalog: Rc<RefCell<AppCatalog>>,
    search: SearchService,
    settings: SettingsService,
    windows: Option<WindowService>,
    /// The logged-in user's lock screen checks their password against.
    username: String,
    locker: gtk4_session_lock::Instance,
    terminals: RefCell<crate::terminal::Terminals>,
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
    topbar: Option<Rc<WebSurface>>,
    preferences: Option<Rc<WebSurface>>,
    finder: Option<Rc<WebSurface>>,
    terminal: Option<Rc<WebSurface>>,
    wallpapers: Vec<(gdk::Monitor, Rc<WebSurface>)>,
    popovers: HashMap<Popover, Rc<WebSurface>>,
    dismiss_timers: HashMap<Popover, glib::SourceId>,
    /// One lock surface per monitor, live only while locked (ADR-0007).
    lock_surfaces: Vec<(gdk::Monitor, Rc<WebSurface>)>,
}

impl Shell {
    pub fn start(app: &gtk::Application, roots: Roots, dev: bool) -> Rc<Shell> {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(crate::desktop::get().appearance == "dark");
            settings.set_gtk_font_name(Some("Inter 11"));
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
            settings: SettingsService::new(),
            windows,
            username: glib::user_name().to_string_lossy().into_owned(),
            locker: gtk4_session_lock::Instance::new(),
            terminals: RefCell::new(crate::terminal::Terminals::default()),
            dev,
            state: RefCell::new(State::default()),
            weak_self: weak_self.clone(),
            _app_monitor: app_monitor.clone(),
            _hold: app.hold(),
        });

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
                shell.state.borrow_mut().lock_surfaces.clear();
            }
        });
        let weak = Rc::downgrade(&shell);
        shell.locker.connect_failed(move |_| {
            log::error!("could not lock the session (unsupported compositor, or already locked elsewhere)");
            if let Some(shell) = weak.upgrade() {
                shell.state.borrow_mut().lock_surfaces.clear();
            }
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
                if let Some(surface) = &shell.state.borrow().preferences {
                    surface.send(&Event::SettingsChanged(state.clone()));
                }
            }
        });

        if let Some(windows) = &shell.windows {
            let weak = Rc::downgrade(&shell);
            windows.connect_changed(move |list| {
                if let Some(shell) = weak.upgrade() {
                    shell.update_dock_visibility(list);
                    let infos = shell.window_infos(list);
                    shell.send_to_popover(Popover::Applications, &Event::WindowsChanged(infos.clone()));
                    shell.send_panel(&Event::WindowsChanged(infos));
                }
            });
        }

        let preferences = WebSurface::new(app, &shell.env, SurfaceKind::Preferences, None, shell.dispatcher());
        shell.state.borrow_mut().preferences = Some(preferences);
        let finder = WebSurface::new(app, &shell.env, SurfaceKind::Finder, None, shell.dispatcher());
        shell.state.borrow_mut().finder = Some(finder);
        let terminal = WebSurface::new(app, &shell.env, SurfaceKind::Terminal, None, shell.dispatcher());
        let weak = shell.weak_self.clone();
        let terminal_weak = Rc::downgrade(&terminal);
        terminal.window.connect_hide(move |_| {
            if let Some(shell) = weak.upgrade() {
                shell.terminals.borrow_mut().clear();
            }
            if let Some(surface) = terminal_weak.upgrade() {
                surface.send(&Event::TerminalClosed);
            }
        });
        shell.state.borrow_mut().terminal = Some(terminal);

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
            Some(shell) => shell.dispatch(surface, request),
            None => Err(ErrorBody::new(ErrorCode::Unavailable, "shell is shutting down")),
        })
    }

    /// Execute an authorized request (capabilities are checked by the bridge).
    fn dispatch(&self, surface: &Rc<WebSurface>, request: Request) -> Result<Reply, ErrorBody> {
        let serialize = |r: Result<String, serde_json::Error>| {
            r.map(Reply::Now).map_err(|e| ErrorBody::new(ErrorCode::Failed, format!("serializing reply: {e}")))
        };
        match request {
            Request::Hello => {
                if surface.kind == SurfaceKind::Terminal {
                    self.terminals.borrow_mut().clear();
                }
                surface.mark_ready();
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
            Request::DesktopAppearanceSet(choice) => {
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let state = crate::files::blocking(move || crate::desktop::set_appearance(&choice.id)).await?;
                    if let Some(shell) = weak.upgrade() {
                        if let Some(settings) = gtk::Settings::default() {
                            settings.set_gtk_application_prefer_dark_theme(state.appearance == "dark");
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
                        "meridian-finder.desktop" => "Finder",
                        "meridian-terminal.desktop" => "Meridian Terminal",
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
                let state = crate::desktop::add_file(&file.path)?;
                self.desktop_changed(&state);
                serialize(proto::reply_ok(&state))
            }
            Request::DesktopChooseFiles => {
                let window = surface.window.clone();
                let weak = self.weak_self.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let dialog = gtk::FileDialog::builder().title("Add files to desktop").build();
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
                            state = crate::desktop::add_file(&file.to_string_lossy())?;
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
                        self.open_finder();
                        if let Some(surface) = &self.state.borrow().finder {
                            surface.send(&Event::FinderLocation(proto::FilePath { path: item.target }));
                        }
                    } else {
                        crate::files::open(&item.target)?;
                    }
                } else {
                    match item.target.as_str() {
                        "meridian-settings.desktop" => self.open_preferences(),
                        "meridian-finder.desktop" => self.open_finder(),
                        "meridian-terminal.desktop" => self.open_terminal(),
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
                if !["Meridian", "File", "Edit", "View", "Window", "Help"].contains(&menu.menu.as_str()) {
                    return Err(ErrorBody::new(ErrorCode::InvalidRequest, "unknown desktop menu"));
                }
                self.open_popover(Popover::DesktopMenu);
                self.send_to_popover(Popover::DesktopMenu, &Event::MenuOpened(menu));
                serialize(ok())
            }
            Request::TerminalOpen => {
                self.open_terminal();
                serialize(ok())
            }
            Request::TerminalOpenAt(path) => {
                let path = crate::files::directory_path(&path.path)?;
                self.open_terminal_at(path.to_string_lossy().into_owned());
                serialize(ok())
            }
            Request::TerminalStart => serialize(proto::reply_ok(&self.terminals.borrow_mut().start()?)),
            Request::TerminalStartAt(path) => {
                serialize(proto::reply_ok(&self.terminals.borrow_mut().start_at(&path.path)?))
            }
            Request::TerminalCopy(text) => {
                if text.data.len() > 1024 * 1024 {
                    return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Clipboard text is too large."));
                }
                gdk::Display::default()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "No clipboard"))?
                    .clipboard()
                    .set_text(&text.data);
                serialize(ok())
            }
            Request::TerminalPaste => {
                let clipboard = gdk::Display::default()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "No clipboard"))?
                    .clipboard();
                Ok(Reply::Later(Box::pin(async move {
                    let text = clipboard
                        .read_text_future()
                        .await
                        .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?
                        .map(|t| t.to_string());
                    if text.as_ref().is_some_and(|t| t.len() > 1024 * 1024) {
                        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Clipboard text is too large."));
                    }
                    proto::reply_ok(&text).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::TerminalRead(session) => {
                serialize(proto::reply_ok(&self.terminals.borrow_mut().read(session.id)?))
            }
            Request::TerminalWrite(input) => {
                self.terminals.borrow_mut().write(input.id, &input.data)?;
                serialize(ok())
            }
            Request::TerminalResize(size) => {
                self.terminals.borrow_mut().resize(size)?;
                serialize(ok())
            }
            Request::TerminalClose(session) => {
                self.terminals.borrow_mut().close(session.id);
                serialize(ok())
            }
            Request::FinderOpen => {
                self.open_finder();
                serialize(ok())
            }
            Request::FileLocations => serialize(proto::reply_ok(&crate::files::locations())),
            Request::FileList(list) => Ok(Reply::Later(Box::pin(async move {
                let result = crate::files::blocking(move || crate::files::list(&list.path, list.hidden)).await?;
                proto::reply_ok(&result).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
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
            Request::TrashOpen => {
                crate::uninstall::open_trash()?;
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
                    && !["meridian-settings.desktop", "meridian-finder.desktop", "meridian-terminal.desktop"]
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
                let query = q.query.trim().to_lowercase();
                let builtins = [
                    ("meridian-settings.desktop", "Settings", "Meridian settings preferences", "preferences-system"),
                    ("meridian-finder.desktop", "Finder", "Meridian finder files folders", "system-file-manager"),
                    (
                        "meridian-terminal.desktop",
                        "Meridian Terminal",
                        "Meridian terminal console command",
                        "utilities-terminal",
                    ),
                ];
                let items = builtins
                    .into_iter()
                    .filter(|(_, _, keywords, _)| keywords.to_lowercase().contains(&query))
                    .map(|(id, name, _, icon)| proto::SearchItem {
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
                let list = self.windows.as_ref().map(WindowService::windows).unwrap_or_default();
                serialize(proto::reply_ok(&self.window_infos(&list)))
            }
            Request::WindowsActivate(a) => {
                let windows = self.windows.as_ref().ok_or_else(|| {
                    ErrorBody::new(ErrorCode::Unavailable, "the compositor does not share its window list")
                })?;
                if !windows.activate(a.id) {
                    return Err(ErrorBody::new(ErrorCode::NotFound, "no such window"));
                }
                serialize(ok())
            }
            Request::SettingsDisplays => Ok(Reply::Later(Box::pin(async move {
                let outputs = crate::files::blocking(|| {
                    meridian_services::displays::list().map_err(|e| ErrorBody::new(ErrorCode::Unavailable, e))
                })
                .await?;
                proto::reply_ok(&outputs).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDisplayApply(choice) => Ok(Reply::Later(Box::pin(async move {
                let trial =
                    crate::files::blocking(move || crate::settings_devices::display_apply(choice.outputs)).await?;
                proto::reply_ok(&trial).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDisplayDecide(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::settings_devices::display_decide(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsMonitorBrightness(choice) => Ok(Reply::Later(Box::pin(async move {
                let value = crate::files::blocking(move || crate::settings_devices::monitor_brightness(choice)).await?;
                proto::reply_ok(&value).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiNetworks => Ok(Reply::Later(Box::pin(async move {
                let networks = crate::files::blocking(crate::settings_devices::wifi_networks).await?;
                proto::reply_ok(&networks).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiConnect(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::settings_devices::wifi_connect(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsBluetoothDevices => Ok(Reply::Later(Box::pin(async move {
                let devices = crate::settings_devices::bluetooth_devices().await?;
                proto::reply_ok(&devices).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsBluetoothScan => Ok(Reply::Later(Box::pin(async move {
                crate::settings_devices::bluetooth_scan().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsBluetoothAction(choice) => {
                let parent = Some(surface.window.clone());
                Ok(Reply::Later(Box::pin(async move {
                    crate::settings_devices::bluetooth_action(choice, parent).await?;
                    proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SettingsGet => serialize(proto::reply_ok(&self.settings.state())),
            Request::SettingsSetWifi(v) => {
                self.settings.set_wifi(v.enabled)?;
                serialize(ok())
            }
            Request::SettingsSetBluetooth(v) => {
                self.settings.set_bluetooth(v.enabled)?;
                serialize(ok())
            }
            Request::SettingsSetBrightness(v) => {
                self.settings.set_brightness(v.percent)?;
                serialize(ok())
            }
            Request::SettingsSetVolume(v) => {
                self.settings.set_volume(v.percent)?;
                serialize(ok())
            }
            Request::SessionLock => {
                self.lock_session()?;
                serialize(ok())
            }
            Request::SessionSignOut => Ok(Reply::Later(Box::pin(async move {
                crate::power::sign_out().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SessionSleep => Ok(Reply::Later(Box::pin(async move {
                crate::power::suspend().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SessionRestart => Ok(Reply::Later(Box::pin(async move {
                crate::power::reboot().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SessionShutdown => Ok(Reply::Later(Box::pin(async move {
                crate::power::power_off().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::LockerUnlock(u) => {
                let weak = self.weak_self.clone();
                let username = self.username.clone();
                Ok(Reply::Later(Box::pin(async move {
                    crate::lock_auth::verify(username, u.password).await?;
                    if let Some(shell) = weak.upgrade() {
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
        if self.locker.is_locked() {
            return Ok(());
        }
        crate::lock_auth::check_setup()?;
        self.env.check_locker().map_err(|message| ErrorBody::new(ErrorCode::Unavailable, message))?;
        if !self.locker.lock() {
            return Err(ErrorBody::new(ErrorCode::Failed, "could not lock the screen"));
        }
        for popover in POPOVERS {
            self.dismiss_popover(popover);
        }
        Ok(())
    }

    fn add_lock_surface(&self, instance: &gtk4_session_lock::Instance, monitor: &gdk::Monitor) {
        let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Locker, Some(monitor), self.dispatcher());
        instance.assign_window_to_monitor(&surface.window, monitor);
        self.state.borrow_mut().lock_surfaces.push((monitor.clone(), surface));
    }

    fn open_terminal(&self) {
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.present();
            surface.send(&Event::TerminalOpened);
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    fn open_terminal_at(&self, path: String) {
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.present();
            surface.send(&Event::TerminalOpenedAt(proto::FilePath { path }));
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
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
        for surface in [&surfaces.preferences, &surfaces.finder, &surfaces.terminal, &surfaces.panel, &surfaces.topbar]
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

    fn open_finder(&self) {
        if let Some(surface) = &self.state.borrow().finder {
            surface.window.present();
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    fn open_preferences(&self) {
        if let Some(surface) = &self.state.borrow().preferences {
            surface.window.present();
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    fn update_dock_visibility(&self, windows: &[Window]) {
        if let Some(panel) = &self.state.borrow().panel {
            panel
                .window
                .set_visible(!self.is_open(Popover::Applications) && !windows.iter().any(|window| window.fullscreen));
        }
    }

    fn window_infos(&self, windows: &[Window]) -> Vec<WindowInfo> {
        let catalog = self.catalog.borrow();
        windows
            .iter()
            .map(|w| WindowInfo {
                id: w.id,
                name: if w.title == "Meridian Terminal" {
                    "Meridian Terminal".into()
                } else if w.title == "Meridian Finder" {
                    "Finder".into()
                } else if w.title == "Meridian Settings" {
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
        self.update_dock_visibility(&self.windows.as_ref().map(|service| service.windows()).unwrap_or_default());
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
                panel.window.present();
                self.state.borrow_mut().panel = Some(panel);
                if let Some(windows) = &self.windows {
                    self.update_dock_visibility(&windows.windows());
                }
                let topbar =
                    WebSurface::new(&self.app, &self.env, SurfaceKind::Topbar, Some(monitor), self.dispatcher());
                topbar.window.present();
                self.state.borrow_mut().topbar = Some(topbar);
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
            let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
            drop.set_propagation_phase(gtk::PropagationPhase::Capture);
            let weak = self.weak_self.clone();
            drop.connect_drop(move |_, value, _, _| {
                let Ok(files) = value.get::<gdk::FileList>() else {
                    return false;
                };
                let Some(shell) = weak.upgrade() else {
                    return false;
                };
                let mut accepted = false;
                for file in files.files() {
                    if let Some(path) = file.path() {
                        match crate::desktop::add_file(&path.to_string_lossy()) {
                            Ok(state) => {
                                shell.desktop_changed(&state);
                                accepted = true;
                            }
                            Err(error) => log::warn!("Desktop drop failed: {error:?}"),
                        }
                    }
                }
                accepted
            });
            wallpaper.window.add_controller(drop);
            wallpaper.window.present();
            self.state.borrow_mut().wallpapers.push((monitor.clone(), wallpaper));
        }
    }

    // ---- actions ---------------------------------------------------------

    /// Fixed, argument-less actions exported on D-Bus for compositor
    /// keybindings: `gapplication action org.meridian.Shell <name>`.
    fn install_actions(&self) {
        let actions: [(&str, ShellAction); 3] = [
            ("toggle-applications", |shell| shell.toggle_popover(Popover::Applications)),
            ("toggle-options", |shell| shell.toggle_popover(Popover::Options)),
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
        Popover::DesktopMenu => SurfaceKind::DesktopMenu,
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

fn dock_path() -> std::path::PathBuf {
    glib::user_config_dir().join("meridian/dock.json")
}

fn read_dock() -> Vec<String> {
    std::fs::read(dock_path()).ok().and_then(|data| serde_json::from_slice(&data).ok()).unwrap_or_default()
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
