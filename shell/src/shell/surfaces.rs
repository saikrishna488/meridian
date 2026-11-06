//! Lazy surface construction and reuse. Shell owns the slots; callbacks hold weak references.
use super::*;

impl Shell {
    pub(super) fn open_utility(&self, kind: SurfaceKind) {
        let existing = {
            let state = self.state.borrow();
            match kind {
                SurfaceKind::Calendar => state.calendar.clone(),
                SurfaceKind::Clock => state.clock.clone(),
                SurfaceKind::Photos => state.photos.clone(),
                SurfaceKind::Media => state.media.clone(),
                _ => return,
            }
        };
        let surface = existing.unwrap_or_else(|| {
            let surface = WebSurface::new(&self.app, &self.env, kind, None, self.dispatcher());
            let mut state = self.state.borrow_mut();
            match kind {
                SurfaceKind::Calendar => state.calendar = Some(surface.clone()),
                SurfaceKind::Clock => state.clock = Some(surface.clone()),
                SurfaceKind::Photos => state.photos = Some(surface.clone()),
                SurfaceKind::Media => state.media = Some(surface.clone()),
                _ => unreachable!(),
            }
            surface
        });
        surface.window.present();
        for popover in POPOVERS { self.hide_popover(popover); }
    }

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

    pub(super) fn open_terminal(&self) { self.present_terminal(None); }
    pub(super) fn open_terminal_at(&self, path: String) { self.present_terminal(Some(path)); }
    fn present_terminal(&self, path: Option<String>) {
        if self.state.borrow().terminal.is_none() {
            let surface = WebSurface::new(&self.app, &self.env, SurfaceKind::Terminal, None, self.dispatcher());
            let weak = self.weak_self.clone();
            let confirming = Rc::new(std::cell::Cell::new(false));
            surface.window.connect_close_request(move |window| {
                let Some(shell) = weak.upgrade() else { return glib::Propagation::Proceed };
                if confirming.get() { return glib::Propagation::Stop; }
                if shell.terminals.borrow_mut().has_jobs() {
                    confirming.set(true);
                    let dialog = gtk::MessageDialog::builder().transient_for(window).modal(true)
                        .message_type(gtk::MessageType::Warning).buttons(gtk::ButtonsType::None)
                        .text("Quit Terminal and stop running jobs?")
                        .secondary_text("All tabs, panes, shells and their child processes will stop. Unsaved work in terminal programs will be lost.").build();
                    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                    dialog.add_button("Quit Terminal", gtk::ResponseType::Accept);
                    dialog.set_default_response(gtk::ResponseType::Cancel);
                    let weak = weak.clone(); let confirming = confirming.clone();
                    dialog.connect_response(move |dialog, response| {
                        confirming.set(false); dialog.destroy();
                        if response == gtk::ResponseType::Accept { if let Some(shell) = weak.upgrade() { shell.finish_terminal(); } }
                    });
                    dialog.present();
                } else { shell.finish_terminal(); }
                glib::Propagation::Stop
            });
            let weak = self.weak_self.clone();
            surface.window.connect_hide(move |_| {
                if let Some(shell) = weak.upgrade() {
                    shell.terminals.borrow_mut().clear();
                    if let Some(surface) = &shell.state.borrow().terminal { surface.send(&Event::TerminalClosed); }
                }
            });
            self.state.borrow_mut().terminal = Some(surface);
        }
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.present();
            if let Some(path) = path { surface.send(&Event::TerminalOpenedAt(proto::FilePath { path })); }
            else { surface.send(&Event::TerminalOpened); }
        }
        for popover in POPOVERS { self.hide_popover(popover); }
    }
    pub(super) fn finish_terminal(&self) {
        self.terminals.borrow_mut().clear();
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.set_visible(false);
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
                Ok(false) => {}
                Err(error) => log::warn!("Default file manager unavailable; opening Meridian Files: {}", error.message),
            }
        }
        self.open_finder();
        if let Some(surface) = &self.state.borrow().finder {
            surface.send(&Event::FinderLocation(proto::FilePath { path: path.to_owned() }));
        }
    }

    pub(super) fn open_engines(&self) {
        self.open_preferences();
        if let Some(surface) = &self.state.borrow().preferences {
            surface
                .send(&Event::PreferencesPage(proto::PreferencesPageRequest { page: proto::PreferencesPage::Engines }));
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
