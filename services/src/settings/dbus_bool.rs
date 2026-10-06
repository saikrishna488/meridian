//! A boolean D-Bus property on the system bus, watched and settable.
//! Used for NetworkManager's `WirelessEnabled` and BlueZ's `Powered`.

use std::cell::RefCell;
use std::rc::Rc;

use gio::prelude::*;

pub struct BoolProperty {
    proxy: Rc<RefCell<Option<gio::DBusProxy>>>,
    name: &'static str,
    path: String,
    interface: &'static str,
    property: &'static str,
}

impl BoolProperty {
    /// Start watching `property`; `on_change` runs once the proxy is ready
    /// and whenever the value changes.
    pub fn watch(
        name: &'static str,
        path: String,
        interface: &'static str,
        property: &'static str,
        on_change: impl Fn() + 'static,
    ) -> Self {
        let slot: Rc<RefCell<Option<gio::DBusProxy>>> = Rc::default();
        let weak = Rc::downgrade(&slot);
        let on_change = Rc::new(on_change);
        gio::DBusProxy::for_bus(
            gio::BusType::System,
            gio::DBusProxyFlags::GET_INVALIDATED_PROPERTIES | gio::DBusProxyFlags::DO_NOT_AUTO_START,
            None,
            name,
            &path,
            interface,
            gio::Cancellable::NONE,
            move |result| {
                let Some(slot) = weak.upgrade() else { return };
                match result {
                    Ok(proxy) => {
                        let notify = on_change.clone();
                        // `connect_local`: main-thread only, closure isn't Send.
                        proxy.connect_local("g-properties-changed", false, move |_| {
                            notify();
                            None
                        });
                        proxy.connect_notify_local(Some("g-name-owner"), {
                            let notify = on_change.clone();
                            move |_, _| notify()
                        });
                        *slot.borrow_mut() = Some(proxy);
                        on_change();
                    }
                    Err(e) => log::warn!("{name}: unavailable: {e}"),
                }
            },
        );
        BoolProperty { proxy: slot, name, path, interface, property }
    }

    /// Current value, or `None` if the service isn't running.
    pub fn get(&self) -> Option<bool> {
        let proxy = self.proxy.borrow();
        let proxy = proxy.as_ref()?;
        proxy.name_owner()?;
        proxy.cached_property(self.property)?.get::<bool>()
    }

    /// Set the property. Failures (e.g. denied by polkit, rfkill-blocked)
    /// are reported to `on_error`.
    pub fn set(&self, value: bool, on_error: impl FnOnce(glib::Error) + 'static) -> bool {
        let Some(proxy) = self.proxy.borrow().clone() else { return false };
        let args = (self.interface, self.property, value.to_variant()).to_variant();
        proxy.connection().call(
            Some(self.name),
            &self.path,
            "org.freedesktop.DBus.Properties",
            "Set",
            Some(&args),
            None,
            gio::DBusCallFlags::NONE,
            -1,
            gio::Cancellable::NONE,
            move |result| {
                if let Err(e) = result {
                    on_error(e);
                }
            },
        );
        true
    }
}
