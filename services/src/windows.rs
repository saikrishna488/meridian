//! Open windows, via `wlr-foreign-toplevel-management-unstable-v1`.
//!
//! Uses its own Wayland connection to the compositor (independent of GTK's),
//! dispatched from the GLib main loop through an fd watch. No polling: the
//! compositor pushes title/app-id/state changes.

use std::cell::RefCell;
use std::os::fd::AsRawFd;
use std::rc::Rc;

use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat::WlSeat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, event_created_child};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    self as handle_v1, ZwlrForeignToplevelHandleV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{
    self as manager_v1, ZwlrForeignToplevelManagerV1,
};

/// A window as seen by the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// Shell-assigned, stable for the window's lifetime.
    pub id: u32,
    pub app_id: String,
    pub title: String,
    pub active: bool,
}

struct Toplevel {
    handle: ZwlrForeignToplevelHandleV1,
    window: Window,
    /// Received its first `done` (all initial properties are known).
    ready: bool,
}

#[derive(Default)]
struct State {
    toplevels: Vec<Toplevel>,
    next_id: u32,
    dirty: bool,
}

type Listener = Box<dyn Fn(&[Window])>;

struct Inner {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    seat: Option<WlSeat>,
    _manager: ZwlrForeignToplevelManagerV1,
    listeners: Vec<Listener>,
}

/// Cheap to clone; all clones share state.
#[derive(Clone)]
pub struct WindowService {
    inner: Rc<RefCell<Inner>>,
}

impl WindowService {
    /// Connect to the compositor named by `$WAYLAND_DISPLAY`. Fails if the
    /// compositor does not offer the foreign-toplevel protocol.
    pub fn connect() -> Result<WindowService, String> {
        let conn = Connection::connect_to_env().map_err(|e| format!("wayland connection: {e}"))?;
        let (globals, mut queue) = registry_queue_init::<State>(&conn).map_err(|e| format!("wayland registry: {e}"))?;
        let qh = queue.handle();
        let manager: ZwlrForeignToplevelManagerV1 = globals
            .bind(&qh, 1..=3, ())
            .map_err(|_| "compositor does not support wlr-foreign-toplevel-management".to_string())?;
        let seat: Option<WlSeat> = globals.bind(&qh, 1..=1, ()).ok();
        if seat.is_none() {
            log::warn!("no wl_seat: windows can be listed but not activated");
        }

        let mut state = State::default();
        queue.roundtrip(&mut state).map_err(|e| format!("wayland roundtrip: {e}"))?;
        state.dirty = false;

        let fd = conn.backend().poll_fd().as_raw_fd();
        let service = WindowService {
            inner: Rc::new(RefCell::new(Inner { conn, queue, state, seat, _manager: manager, listeners: Vec::new() })),
        };
        let weak = Rc::downgrade(&service.inner);
        glib_unix::unix_fd_add_local(
            fd,
            glib::IOCondition::IN | glib::IOCondition::HUP | glib::IOCondition::ERR,
            move |_, cond| {
                let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
                let service = WindowService { inner };
                if cond.intersects(glib::IOCondition::HUP | glib::IOCondition::ERR) {
                    log::error!("compositor connection for the window list closed");
                    service.inner.borrow_mut().state.toplevels.clear();
                    service.notify();
                    return glib::ControlFlow::Break;
                }
                service.dispatch();
                glib::ControlFlow::Continue
            },
        );
        Ok(service)
    }

    fn dispatch(&self) {
        let changed = {
            let mut inner = self.inner.borrow_mut();
            let Inner { conn, queue, state, .. } = &mut *inner;
            if let Some(guard) = queue.prepare_read()
                && let Err(e) = guard.read()
            {
                log::debug!("wayland read: {e}");
            }
            if let Err(e) = queue.dispatch_pending(state) {
                log::error!("wayland dispatch: {e}");
            }
            if let Err(e) = conn.flush() {
                log::warn!("wayland flush: {e}");
            }
            std::mem::take(&mut state.dirty)
        };
        if changed {
            self.notify();
        }
    }

    fn notify(&self) {
        let windows = self.windows();
        for listener in &self.inner.borrow().listeners {
            listener(&windows);
        }
    }

    /// Open windows, oldest first.
    pub fn windows(&self) -> Vec<Window> {
        self.inner.borrow().state.toplevels.iter().filter(|t| t.ready).map(|t| t.window.clone()).collect()
    }

    /// Focus a window. Returns false if no such window exists.
    pub fn activate(&self, id: u32) -> bool {
        let inner = self.inner.borrow();
        let Some(toplevel) = inner.state.toplevels.iter().find(|t| t.window.id == id) else { return false };
        match &inner.seat {
            Some(seat) => {
                toplevel.handle.activate(seat);
                if let Err(e) = inner.conn.flush() {
                    log::warn!("wayland flush: {e}");
                }
            }
            None => log::warn!("cannot activate window {id}: no seat"),
        }
        true
    }

    /// Listeners must not register further listeners.
    pub fn connect_changed(&self, f: impl Fn(&[Window]) + 'static) {
        self.inner.borrow_mut().listeners.push(Box::new(f));
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Globals appearing later (e.g. a second seat) are not needed.
    }
}

impl Dispatch<WlSeat, ()> for State {
    fn event(_: &mut Self, _: &WlSeat, _: <WlSeat as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            manager_v1::Event::Toplevel { toplevel } => {
                let id = state.next_id;
                state.next_id += 1;
                state.toplevels.push(Toplevel {
                    handle: toplevel,
                    window: Window { id, app_id: String::new(), title: String::new(), active: false },
                    ready: false,
                });
            }
            manager_v1::Event::Finished => {
                log::warn!("compositor stopped sending window updates");
                state.toplevels.clear();
                state.dirty = true;
            }
            _ => {}
        }
    }

    event_created_child!(State, ZwlrForeignToplevelManagerV1, [
        manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let handle_v1::Event::Closed = event {
            handle.destroy();
            state.toplevels.retain(|t| &t.handle != handle);
            state.dirty = true;
            return;
        }
        let Some(t) = state.toplevels.iter_mut().find(|t| &t.handle == handle) else { return };
        match event {
            handle_v1::Event::Title { title } => t.window.title = title,
            handle_v1::Event::AppId { app_id } => t.window.app_id = app_id,
            handle_v1::Event::State { state: raw } => t.window.active = is_activated(&raw),
            // Properties are applied atomically on `done`.
            handle_v1::Event::Done => {
                t.ready = true;
                state.dirty = true;
            }
            _ => {}
        }
    }
}

/// The `state` event carries an array of native-endian u32 state values.
fn is_activated(raw: &[u8]) -> bool {
    raw.as_chunks::<4>().0.iter().map(|c| u32::from_ne_bytes(*c)).any(|s| s == handle_v1::State::Activated as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_state_array() {
        let maximized = (handle_v1::State::Maximized as u32).to_ne_bytes();
        let activated = (handle_v1::State::Activated as u32).to_ne_bytes();
        assert!(!is_activated(&maximized));
        assert!(is_activated(&[maximized, activated].concat()));
        assert!(!is_activated(&[]));
    }
}
