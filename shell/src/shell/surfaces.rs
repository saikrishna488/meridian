//! Lazy surface construction and reuse. Shell owns the slots; callbacks hold weak references.
use super::*;

impl Shell {
    pub(super) fn open_utility(&self, kind: SurfaceKind) {
        let mut state = self.state.borrow_mut();
        let slot = match kind {
            SurfaceKind::Nova => &mut state.nova,
            SurfaceKind::Calendar => &mut state.calendar,
            _ => &mut state.clock,
        };
        if slot.is_none() {
            *slot = Some(WebSurface::new(&self.app, &self.env, kind, None, self.dispatcher()));
        }
        slot.as_ref().unwrap().window.present();
        drop(state);
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
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

    pub(super) fn ensure_terminal(&self) {
        if self.state.borrow().terminal.is_some() {
            return;
        }
        let terminal = WebSurface::new(&self.app, &self.env, SurfaceKind::Terminal, None, self.dispatcher());
        let weak = self.weak_self.clone();
        let terminal_weak = Rc::downgrade(&terminal);
        terminal.window.connect_hide(move |_| {
            if let Some(shell) = weak.upgrade() {
                shell.terminals.borrow_mut().clear();
            }
            if let Some(surface) = terminal_weak.upgrade() {
                surface.send(&Event::TerminalClosed);
            }
        });
        self.state.borrow_mut().terminal = Some(terminal);
    }

    pub(super) fn open_terminal(&self) {
        self.ensure_terminal();
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.present();
            surface.send(&Event::TerminalOpened);
        }
        for popover in POPOVERS {
            self.hide_popover(popover);
        }
    }

    pub(super) fn open_terminal_at(&self, path: String) {
        self.ensure_terminal();
        if let Some(surface) = &self.state.borrow().terminal {
            surface.window.present();
            surface.send(&Event::TerminalOpenedAt(proto::FilePath { path }));
        }
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
