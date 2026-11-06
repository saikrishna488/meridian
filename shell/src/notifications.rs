//! Session notification server implementing org.freedesktop.Notifications.
use glib::variant::ToVariant;
use gtk::prelude::*;
use gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode};
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
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Rules { muted_apps: Vec<String>, quiet_until: Option<f64> }
pub struct Notifications {
    rules: RefCell<Rules>,
    toast: RefCell<Option<(u32, gtk::Window)>>,
    locked: Cell<bool>,
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
        let rules = crate::config::read_versioned::<Rules>(&preferences_path.with_file_name("notification-rules.json")).unwrap_or_default();
        let service = Rc::new(Self {
            rules: RefCell::new(rules), toast: RefCell::new(None), locked: Cell::new(false),
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
        let weak = Rc::downgrade(&service);
        glib::timeout_add_local(Duration::from_secs(15), move || {
            let Some(service) = weak.upgrade() else { return glib::ControlFlow::Break };
            if service.rules.borrow().quiet_until.is_some_and(|until| until <= now()) {
                service.rules.borrow_mut().quiet_until = None;
                service.publish();
            }
            glib::ControlFlow::Continue
        });
        service
    }
    pub fn snapshot(&self) -> NotificationState {
        NotificationState {
            available: self.available.get(),
            do_not_disturb: self.quiet(),
            muted_apps: self.rules.borrow().muted_apps.clone(),
            quiet_until: self.rules.borrow().quiet_until.filter(|until| *until > now()),
            items: self.entries.borrow().iter().rev().map(|e| e.item.clone()).collect(),
        }
    }
    fn quiet(&self) -> bool { self.do_not_disturb.get() || self.rules.borrow().quiet_until.is_some_and(|until| until > now()) }
    fn hide_toast(&self) { if let Some((_,window)) = self.toast.borrow_mut().take() { window.destroy(); } }
    pub fn set_locked(&self, locked: bool) { self.locked.set(locked); if locked { self.hide_toast(); } }
    pub fn configure(&self, change: meridian_protocol::WorkflowChange) -> Result<(), meridian_protocol::ErrorBody> {
        let mut rules = self.rules.borrow().clone();
        match change.action.as_str() {
            "quiet" => {
                let minutes = change.value.parse::<u32>().ok().filter(|n| [0,15,30,60,120].contains(n))
                    .ok_or_else(|| meridian_protocol::ErrorBody::new(meridian_protocol::ErrorCode::InvalidRequest,"Choose a supported quiet duration"))?;
                rules.quiet_until = (minutes > 0).then(|| now() + f64::from(minutes) * 60_000.0);
            }
            "mute-app" => {
                if change.id.is_empty() || change.id.chars().count() > 128 { return Err(meridian_protocol::ErrorBody::new(meridian_protocol::ErrorCode::InvalidRequest,"Invalid application name")); }
                rules.muted_apps.retain(|app| app != &change.id);
                if change.value == "true" {
                    if rules.muted_apps.len() >= 100 { return Err(meridian_protocol::ErrorBody::new(meridian_protocol::ErrorCode::InvalidRequest,"Too many application rules")); }
                    rules.muted_apps.push(change.id);
                }
            }
            _ => return Err(meridian_protocol::ErrorBody::new(meridian_protocol::ErrorCode::InvalidRequest,"Unknown notification preference")),
        }
        crate::config::save_versioned(&self.preferences_path.with_file_name("notification-rules.json"), &rules)?;
        if change.action == "quiet" {
            if let Err(error) = crate::config::save_versioned(&self.preferences_path, &false) {
                let _ = crate::config::save_versioned(&self.preferences_path.with_file_name("notification-rules.json"), &*self.rules.borrow());
                return Err(error);
            }
            self.do_not_disturb.set(false);
        }
        *self.rules.borrow_mut() = rules;
        let quiet = self.quiet();
        for entry in self.entries.borrow_mut().iter_mut() {
            if quiet || self.rules.borrow().muted_apps.contains(&entry.item.app) { entry.item.unread = false; }
        }
        self.hide_toast(); self.publish(); Ok(())
    }
    fn show_toast(self: &Rc<Self>, item: &NotificationItem) {
        if self.locked.get() || self.quiet() || self.rules.borrow().muted_apps.contains(&item.app) { return; }
        self.hide_toast();
        let window = gtk::Window::builder().title("Notification").default_width(360).resizable(false).decorated(false).build();
        window.init_layer_shell(); window.set_layer(Layer::Top); window.set_namespace(Some("meridian-toast"));
        window.set_anchor(Edge::Top, true); window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Top, 56); window.set_margin(Edge::Right, 16); window.set_keyboard_mode(KeyboardMode::OnDemand);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        content.set_margin_top(16); content.set_margin_bottom(16); content.set_margin_start(16); content.set_margin_end(16);
        let app = gtk::Label::new(Some(&item.app)); app.set_xalign(0.0); content.append(&app);
        let title = gtk::Label::new(Some(&item.summary)); title.set_wrap(true); title.set_max_width_chars(42); title.set_wrap_mode(gtk::pango::WrapMode::WordChar); title.set_xalign(0.0); title.add_css_class("heading"); content.append(&title);
        let body = gtk::Label::new(Some(&item.body.chars().take(240).collect::<String>())); body.set_wrap(true); body.set_max_width_chars(42); body.set_wrap_mode(gtk::pango::WrapMode::WordChar); body.set_xalign(0.0); content.append(&body);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for (key,label) in item.actions.iter().take(3) {
            let button = gtk::Button::with_label(label); let weak = Rc::downgrade(self); let id = item.id; let key = key.clone();
            button.connect_clicked(move |_| { if let Some(service) = weak.upgrade() { service.activate(id,&key); } }); actions.append(&button);
        }
        let dismiss = gtk::Button::with_label("Dismiss"); let weak = Rc::downgrade(self); let id = item.id;
        dismiss.connect_clicked(move |_| { if let Some(service) = weak.upgrade() { service.dismiss(id); } }); actions.append(&dismiss);
        content.append(&actions); window.set_child(Some(&content));
        // Present without requesting focus; keyboard access remains in notification history.
        window.set_visible(true); *self.toast.borrow_mut() = Some((item.id, window));
        let weak = Rc::downgrade(self); let id = item.id; let time = item.time;
        glib::timeout_add_local_once(Duration::from_secs(8), move || {
            if let Some(service) = weak.upgrade() {
                let same = service.entries.borrow().iter().any(|entry| entry.item.id == id && entry.item.time == time);
                if same && service.toast.borrow().as_ref().is_some_and(|(current,_)| *current == id) { service.hide_toast(); }
            }
        });
    }
    pub fn set_do_not_disturb(&self, enabled: bool) -> Result<(), meridian_protocol::ErrorBody> {
        crate::config::save_versioned(&self.preferences_path, &enabled)?;
        if !enabled {
            let mut rules = self.rules.borrow().clone(); rules.quiet_until = None;
            crate::config::save_versioned(&self.preferences_path.with_file_name("notification-rules.json"), &rules)?;
            *self.rules.borrow_mut() = rules;
        }
        self.do_not_disturb.set(enabled);
        if enabled { self.hide_toast(); }
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
        if self.toast.borrow().as_ref().is_some_and(|(current,_)| *current == id) { self.hide_toast(); }
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
        self.hide_toast();
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
                    unread: !self.quiet() && !self.rules.borrow().muted_apps.contains(&string(0).chars().take(128).collect::<String>()),
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
                self.show_toast(&item);
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
        if let Some((_,window)) = self.toast.get_mut().take() { window.destroy(); }
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
            rules: RefCell::default(), toast: RefCell::default(), locked: Cell::new(false),
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

fn now() -> f64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64() * 1000.0 }
