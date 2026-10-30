//! Lazy surface construction and reuse. Shell owns the slots; callbacks hold weak references.
use super::*;

impl Shell {
    pub(super) fn open_pulse(&self) {
        if self.state.borrow().pulse.is_none() {
            let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Pulse, None, self.dispatcher());
            let weak = Rc::downgrade(&surface);
            surface.window.connect_hide(move |_| {
                if let Some(surface) = weak.upgrade() {
                    surface.send(&Event::PulseVisibility(false));
                }
            });
            self.state.borrow_mut().pulse = Some(surface);
        }
        if let Some(surface) = &self.state.borrow().pulse {
            surface.window.present();
            surface.send(&Event::PulseVisibility(true));
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn open_meridian_terminal(&self) {
        if self.state.borrow().terminal.is_none() {
            let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Terminal, None, self.dispatcher());
            self.state.borrow_mut().terminal = Some(surface);
        }
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.present();
            if surface.is_ready() {
                surface.send(&Event::TerminalOpened);
            }
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn open_terminal(&self) {
        if crate::default_apps::preferred("terminal").as_deref() == Some("meridian-terminal.desktop") {
            self.open_meridian_terminal();
            return;
        }
        if !crate::default_apps::launch_terminal() { launch_kitty(None); }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn open_terminal_at(&self, path: String) {
        launch_kitty(Some(path));
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn open_finder(&self) {
        if self.state.borrow().finder.is_none() {
            let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Finder, None, self.dispatcher());
            self.state.borrow_mut().finder = Some(surface);
        }
        if let Some(surface) = &self.state.borrow().finder {
            surface.window.present();
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn open_finder_at(&self, path: &str) {
        if !path.starts_with("@") {
            match crate::default_apps::launch_file_manager(path) {
                Ok(true) => return,
                Ok(false) => {},
                Err(error) => log::warn!("Default file manager unavailable; opening Meridian Files: {}", error.message),
            }
        }
        self.open_finder();
        if let Some(surface) = &self.state.borrow().finder {
            surface.send(&Event::FinderLocation(proto::FilePath { path: path.to_owned() }));
        }
    }

    pub(super) fn open_preferences(&self) {
        if self.state.borrow().preferences.is_none() {
            let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Preferences, None, self.dispatcher());
            self.state.borrow_mut().preferences = Some(surface);
        }
        if let Some(surface) = &self.state.borrow().preferences {
            surface.window.present();
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn ensure_popover(&self, popover: Popover) -> Rc<WebSurface> {
        if let Some(surface) = self.state.borrow().popovers.get(&popover).cloned() {
            return surface;
        }
        let surface = WebSurface::new(&self.app, &self.env, popover_surface(popover), None, self.dispatcher());
        self.state.borrow_mut().popovers.insert(popover, surface.clone());
        surface
    }
}

fn launch_kitty(cwd: Option<String>) {
    let mut argv: Vec<std::ffi::OsString> = Vec::new();
    if std::env::var("MERIDIAN_HOST_APPS").as_deref() == Ok("1") {
        argv.extend(["flatpak-spawn".into(), "--host".into()]);
    }
    argv.push("kitty".into());
    if let Some(path) = cwd {
        argv.extend(["--directory".into(), path.into()]);
    }
    let refs: Vec<_> = argv.iter().map(|arg| arg.as_os_str()).collect();
    match gtk::gio::Subprocess::newv(&refs, gtk::gio::SubprocessFlags::NONE) {
        Ok(process) => process.wait_check_async(gtk::gio::Cancellable::NONE, |result| {
            if let Err(error) = result {
                log::warn!("Kitty failed to start: {error}");
            }
        }),
        Err(error) => log::warn!("Could not start Kitty: {error}"),
    }
}
