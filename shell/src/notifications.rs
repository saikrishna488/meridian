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
    expiry: Option<glib::SourceId>,
}
pub struct Notifications {
    entries: RefCell<Vec<Entry>>,
    connection: RefCell<Option<gio::DBusConnection>>,
    owner: RefCell<Option<gio::OwnerId>>,
    registration: RefCell<Option<gio::RegistrationId>>,
    available: Cell<bool>,
    next: Cell<u32>,
    generation: Cell<u64>,
    do_not_disturb: Cell<bool>,
    preferences_path: std::path::PathBuf,
    sender_subscription: RefCell<Option<gio::SignalSubscription>>,
    changed: Box<dyn Fn(NotificationState)>,
}
impl Notifications {
    pub fn start(changed: impl Fn(NotificationState) + 'static) -> Rc<Self> {
        let preferences_path = glib::user_config_dir().join("meridian/notification-preferences.json");
        let do_not_disturb = crate::config::read_versioned::<bool>(&preferences_path).unwrap_or(false);
        let service = Rc::new(Self {
            do_not_disturb: Cell::new(do_not_disturb),
            preferences_path,
            sender_subscription: RefCell::default(),
            entries: RefCell::new(vec![]),
            connection: RefCell::new(None),
            owner: RefCell::new(None),
            registration: RefCell::new(None),
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
                    match registration {
                        Ok(registration) => {
                            if let Some(old) = service.registration.borrow_mut().replace(registration)
                                && let Some(connection) = service.connection.borrow().as_ref()
                            {
                                let _ = connection.unregister_object(old);
                            }
                            let weak = Rc::downgrade(&service);
                            let subscription = bus.subscribe_to_signal(
                                Some("org.freedesktop.DBus"),
                                Some("org.freedesktop.DBus"),
                                Some("NameOwnerChanged"),
                                Some("/org/freedesktop/DBus"),
                                None,
                                gio::DBusSignalFlags::NONE,
                                move |signal| {
                                    if let Some((name, _, owner)) = signal.parameters.get::<(String, String, String)>()
                                        && name.starts_with(':')
                                        && owner.is_empty()
                                        && let Some(service) = weak.upgrade()
                                    {
                                        service.sender_disconnected(&name);
                                    }
                                },
                            );
                            *service.sender_subscription.borrow_mut() = Some(subscription);
                            *service.connection.borrow_mut() = Some(bus);
                        }
                        Err(_) => log::warn!("component=notifications event=registration_failed"),
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
            do_not_disturb: self.do_not_disturb.get(),
            items: self.entries.borrow().iter().rev().map(|e| e.item.clone()).collect(),
        }
    }
    pub fn set_do_not_disturb(&self, enabled: bool) -> Result<(), meridian_protocol::ErrorBody> {
        crate::config::save_versioned(&self.preferences_path, &enabled)?;
        self.do_not_disturb.set(enabled);
        if enabled {
            for entry in self.entries.borrow_mut().iter_mut() {
                entry.item.unread = false;
            }
        }
        self.publish();
        Ok(())
    }
    fn sender_disconnected(&self, sender: &str) {
        let mut changed = false;
        for entry in self.entries.borrow_mut().iter_mut().filter(|entry| entry.sender == sender && entry.active) {
            entry.active = false;
            if let Some(timer) = entry.expiry.take() {
                timer.remove();
            }
            entry.item.actions.clear();
            changed = true;
        }
        if changed {
            self.publish();
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
            if let Some(timer) = entry.expiry.take() {
                timer.remove();
            }
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
            if let Some(timer) = entry.expiry {
                timer.remove();
            }
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
                    .as_chunks::<2>()
                    .0
                    .iter()
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
                    unread: !self.do_not_disturb.get(),
                    actions,
                };
                let mut entries = self.entries.borrow_mut();
                if let Some(index) = entries.iter().position(|e| e.item.id == id)
                    && let Some(timer) = entries.remove(index).expiry
                {
                    timer.remove();
                }
                drop(entries);
                if self.entries.borrow().len() >= 100 {
                    let old = self.entries.borrow()[0].item.id;
                    self.close(old, 4, true);
                }
                self.entries.borrow_mut().push(Entry {
                    item,
                    sender: sender.into(),
                    active: true,
                    generation,
                    expiry: None,
                });
                self.publish();
                invocation.return_value(Some(&(id,).to_variant()));
                let expiry = params.child_value(7).get::<i32>().unwrap_or(-1);
                // Keep a history entry after expiry, but close its action channel.
                if expiry != 0 {
                    let ms = if expiry < 0 { 5000 } else { expiry.min(86_400_000) as u64 };
                    let weak = Rc::downgrade(self);
                    let timer = glib::timeout_add_local_once(Duration::from_millis(ms), move || {
                        if let Some(service) = weak.upgrade() {
                            let current = service
                                .entries
                                .borrow()
                                .iter()
                                .any(|e| e.item.id == id && e.generation == generation && e.active);
                            if current {
                                if let Some(entry) = service.entries.borrow_mut().iter_mut().find(|e| e.item.id == id) {
                                    entry.expiry.take();
                                }
                                service.close(id, 1, false);
                            }
                        }
                    });
                    if let Some(entry) = self.entries.borrow_mut().iter_mut().find(|e| e.item.id == id) {
                        entry.expiry = Some(timer);
                    }
                }
            }
            _ => invocation.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", "Unknown method"),
        }
    }
}

impl Drop for Notifications {
    fn drop(&mut self) {
        for entry in self.entries.get_mut() {
            if let Some(timer) = entry.expiry.take() {
                timer.remove();
            }
        }
        self.sender_subscription.get_mut().take();
        if let Some(id) = self.registration.get_mut().take()
            && let Some(connection) = self.connection.get_mut().as_ref()
        {
            let _ = connection.unregister_object(id);
        }
        if let Some(owner) = self.owner.get_mut().take() {
            gio::bus_unown_name(owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn service() -> Notifications {
        Notifications {
            do_not_disturb: Cell::new(false),
            preferences_path: std::env::temp_dir()
                .join(format!("meridian-notification-preferences-test-{}.json", std::process::id())),
            sender_subscription: RefCell::default(),
            entries: RefCell::new(vec![]),
            connection: RefCell::new(None),
            owner: RefCell::new(None),
            registration: RefCell::new(None),
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
            expiry: None,
        }
    }
    #[test]
    fn dnd_silences_badges_and_disconnected_senders_lose_actions() {
        let service = service();
        service.entries.borrow_mut().push(entry(1));
        service.set_do_not_disturb(true).unwrap();
        assert!(service.snapshot().do_not_disturb);
        assert!(!service.snapshot().items[0].unread);
        assert_eq!(crate::config::read_versioned::<bool>(&service.preferences_path), Some(true));
        service.sender_disconnected(":1.2");
        assert!(service.snapshot().items[0].actions.is_empty());
        assert!(!service.entries.borrow()[0].active);
        std::fs::remove_file(&service.preferences_path).unwrap();
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
