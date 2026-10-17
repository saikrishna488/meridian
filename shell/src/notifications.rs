//! Session notification server implementing org.freedesktop.Notifications.
use glib::variant::ToVariant;
use gtk::{gio, glib};
use meridian_protocol::{NotificationItem, NotificationState};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
const XML: &str = r#"<node><interface name="org.freedesktop.Notifications">
<method name="GetCapabilities"><arg type="as" direction="out"/></method>
<method name="GetServerInformation"><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="s" direction="out"/></method>
<method name="Notify"><arg type="s" direction="in"/><arg type="u" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="as" direction="in"/><arg type="a{sv}" direction="in"/><arg type="i" direction="in"/><arg type="u" direction="out"/></method>
<method name="CloseNotification"><arg type="u" direction="in"/></method>
<signal name="NotificationClosed"><arg type="u"/><arg type="u"/></signal>
<signal name="ActionInvoked"><arg type="u"/><arg type="s"/></signal>
</interface></node>"#;
struct Entry {
    item: NotificationItem,
    sender: String,
    active: bool,
    generation: u64,
}
pub struct Notifications {
    entries: RefCell<Vec<Entry>>,
    connection: RefCell<Option<gio::DBusConnection>>,
    owner: RefCell<Option<gio::OwnerId>>,
    available: Cell<bool>,
    next: Cell<u32>,
    generation: Cell<u64>,
    changed: Box<dyn Fn(NotificationState)>,
}
impl Notifications {
    pub fn start(changed: impl Fn(NotificationState) + 'static) -> Rc<Self> {
        let service = Rc::new(Self {
            entries: RefCell::new(vec![]),
            connection: RefCell::new(None),
            owner: RefCell::new(None),
            available: Cell::new(false),
            next: Cell::new(1),
            generation: Cell::new(0),
            changed: Box::new(changed),
        });
        let weak = Rc::downgrade(&service);
        let acquired = weak.clone();
        let lost = weak.clone();
        let owner = gio::bus_own_name(
            gio::BusType::Session,
            NAME,
            gio::BusNameOwnerFlags::DO_NOT_QUEUE,
            move |bus, _| {
                if let Some(service) = weak.upgrade() {
                    let node = gio::DBusNodeInfo::for_xml(XML).expect("notification interface XML");
                    let interface = node.lookup_interface(NAME).expect("notification interface");
                    let weak = Rc::downgrade(&service);
                    let registration = bus
                        .register_object(PATH, &interface)
                        .method_call(move |_, sender, _, _, method, params, invocation| {
                            if let Some(service) = weak.upgrade() {
                                service.method(sender.unwrap_or_default(), method, params, invocation);
                            } else {
                                invocation.return_dbus_error("org.freedesktop.DBus.Error.Failed", "Service stopped");
                            }
                        })
                        .build();
                    if registration.is_ok() {
                        *service.connection.borrow_mut() = Some(bus);
                    } else {
                        log::warn!("Could not register notification service");
                    }
                }
            },
            move |_, _| {
                if let Some(service) = acquired.upgrade() {
                    service.available.set(service.connection.borrow().is_some());
                    service.publish();
                }
            },
            move |_, _| {
                if let Some(service) = lost.upgrade() {
                    service.available.set(false);
                    service.publish();
                }
            },
        );
        *service.owner.borrow_mut() = Some(owner);
        service
    }
    pub fn snapshot(&self) -> NotificationState {
        NotificationState {
            available: self.available.get(),
            items: self.entries.borrow().iter().rev().map(|e| e.item.clone()).collect(),
        }
    }
    fn publish(&self) {
        (self.changed)(self.snapshot());
    }
    fn signal(&self, sender: &str, signal: &str, params: glib::Variant) {
        if let Some(bus) = self.connection.borrow().as_ref() {
            let _ = bus.emit_signal(Some(sender), PATH, NAME, signal, Some(&params));
        }
    }
    fn close(&self, id: u32, reason: u32, remove: bool) {
        let mut entries = self.entries.borrow_mut();
        if let Some(index) = entries.iter().position(|e| e.item.id == id) {
            let entry = &mut entries[index];
            let active = entry.active;
            entry.active = false;
            if active {
                self.signal(&entry.sender, "NotificationClosed", (id, reason).to_variant());
            }
            entry.item.actions.clear();
            if remove {
                entries.remove(index);
            }
        }
        drop(entries);
        self.publish();
    }
    pub fn dismiss(&self, id: u32) {
        self.close(id, 2, true);
    }
    pub fn clear(&self) {
        let entries = std::mem::take(&mut *self.entries.borrow_mut());
        for entry in entries {
            if entry.active {
                self.signal(&entry.sender, "NotificationClosed", (entry.item.id, 2u32).to_variant());
            }
        }
        self.publish();
    }
    pub fn mark_read(&self) {
        let mut changed = false;
        for entry in self.entries.borrow_mut().iter_mut() {
            changed |= entry.item.unread;
            entry.item.unread = false;
        }
        if changed {
            self.publish();
        }
    }
    pub fn activate(&self, id: u32, key: &str) {
        let sender = self
            .entries
            .borrow()
            .iter()
            .find(|e| e.item.id == id && e.active && e.item.actions.iter().any(|(k, _)| k == key))
            .map(|e| e.sender.clone());
        if let Some(sender) = sender {
            self.signal(&sender, "ActionInvoked", (id, key).to_variant());
            self.dismiss(id);
        }
    }
    fn method(
        self: &Rc<Self>,
        sender: &str,
        method: &str,
        params: glib::Variant,
        invocation: gio::DBusMethodInvocation,
    ) {
        match method {
            "GetCapabilities" => invocation.return_value(Some(&(vec!["body", "actions", "persistence"],).to_variant())),
            "GetServerInformation" => {
                invocation.return_value(Some(&("Meridian", "Meridian", env!("CARGO_PKG_VERSION"), "1.2").to_variant()))
            }
            "CloseNotification" => {
                let id = params.child_value(0).get::<u32>().unwrap_or(0);
                let owned = self.entries.borrow().iter().any(|e| e.item.id == id && e.sender == sender);
                if owned {
                    self.close(id, 3, true);
                    invocation.return_value(Some(&().to_variant()));
                } else {
                    invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", "");
                }
            }
            "Notify" => {
                let string = |i| params.child_value(i).get::<String>().unwrap_or_default();
                let replace = params.child_value(1).get::<u32>().unwrap_or(0);
                let id = if self.entries.borrow().iter().any(|e| e.item.id == replace && e.sender == sender && e.active)
                {
                    replace
                } else {
                    let id = self.next.get();
                    self.next.set(id.wrapping_add(1).max(1));
                    id
                };
                let actions = params.child_value(5).get::<Vec<String>>().unwrap_or_default();
                let actions = actions
                    .chunks_exact(2)
                    .take(8)
                    .map(|a| (a[0].chars().take(128).collect(), a[1].chars().take(128).collect()))
                    .collect();
                let generation = self.generation.get().wrapping_add(1);
                self.generation.set(generation);
                let item = NotificationItem {
                    id,
                    app: string(0).chars().take(128).collect(),
                    summary: string(3).chars().take(256).collect(),
                    body: string(4).chars().take(4096).collect(),
                    time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() * 1000.0,
                    unread: true,
                    actions,
                };
                self.entries.borrow_mut().retain(|e| e.item.id != id);
                if self.entries.borrow().len() >= 100 {
                    let old = self.entries.borrow()[0].item.id;
                    self.close(old, 4, true);
                }
                self.entries.borrow_mut().push(Entry { item, sender: sender.into(), active: true, generation });
                self.publish();
                invocation.return_value(Some(&(id,).to_variant()));
                let expiry = params.child_value(7).get::<i32>().unwrap_or(-1);
                // Keep a history entry after expiry, but close its action channel.
                if expiry > 0 {
                    let ms = expiry.min(86_400_000) as u64;
                    let weak = Rc::downgrade(self);
                    glib::timeout_add_local_once(Duration::from_millis(ms), move || {
                        if let Some(service) = weak.upgrade() {
                            let current = service
                                .entries
                                .borrow()
                                .iter()
                                .any(|e| e.item.id == id && e.generation == generation && e.active);
                            if current {
                                service.close(id, 1, false);
                            }
                        }
                    });
                }
            }
            _ => invocation.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", "Unknown method"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn service() -> Notifications {
        Notifications {
            entries: RefCell::new(vec![]),
            connection: RefCell::new(None),
            owner: RefCell::new(None),
            available: Cell::new(true),
            next: Cell::new(1),
            generation: Cell::new(0),
            changed: Box::new(|_| {}),
        }
    }
    fn entry(id: u32) -> Entry {
        Entry {
            item: NotificationItem {
                id,
                app: "Test".into(),
                summary: "A message".into(),
                body: "Body".into(),
                time: 0.0,
                unread: true,
                actions: vec![("open".into(), "Open".into())],
            },
            sender: ":1.2".into(),
            active: true,
            generation: 1,
        }
    }
    #[test]
    fn clearing_history_publishes_one_update_and_reading_read_items_is_quiet() {
        let count = Rc::new(Cell::new(0));
        let published = count.clone();
        let mut service = service();
        service.changed = Box::new(move |_| published.set(published.get() + 1));
        service.entries.borrow_mut().extend([entry(1), entry(2)]);
        service.mark_read();
        assert_eq!(count.get(), 1);
        service.mark_read();
        assert_eq!(count.get(), 1);
        service.clear();
        assert_eq!(count.get(), 2);
        assert!(service.snapshot().items.is_empty());
    }
    #[test]
    fn history_unread_expiry_actions_and_dismissal() {
        let service = service();
        service.entries.borrow_mut().extend([entry(1), entry(2)]);
        assert_eq!(service.snapshot().items[0].id, 2);
        service.mark_read();
        assert!(service.snapshot().items.iter().all(|i| !i.unread));
        service.close(1, 1, false);
        assert!(service.snapshot().items.iter().find(|i| i.id == 1).unwrap().actions.is_empty());
        service.activate(2, "untrusted");
        assert_eq!(service.snapshot().items.len(), 2);
        service.activate(2, "open");
        assert_eq!(service.snapshot().items.len(), 1);
        service.clear();
        assert!(service.snapshot().items.is_empty());
    }
    #[test]
    fn exports_the_standard_notification_interface() {
        let node = gio::DBusNodeInfo::for_xml(XML).unwrap();
        let interface = node.lookup_interface(NAME).unwrap();
        for method in ["Notify", "CloseNotification", "GetCapabilities", "GetServerInformation"] {
            assert!(interface.lookup_method(method).is_some());
        }
        for signal in ["NotificationClosed", "ActionInvoked"] {
            assert!(interface.lookup_signal(signal).is_some());
        }
    }
}
