//! A temporary BlueZ agent for pairing initiated from Meridian Settings.
use gtk::prelude::*;
use gtk::{gio, glib};
use meridian_protocol::{ErrorBody, ErrorCode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Prompt {
    _window: gtk::Window,
    finish: Rc<dyn Fn(bool)>,
    notification: bool,
}
const PATH: &str = "/org/meridian/PairingAgent";
const XML: &str = r#"<node><interface name="org.bluez.Agent1">
<method name="Release"/><method name="Cancel"/>
<method name="RequestPinCode"><arg type="o" direction="in"/><arg type="s" direction="out"/></method>
<method name="RequestPasskey"><arg type="o" direction="in"/><arg type="u" direction="out"/></method>
<method name="DisplayPinCode"><arg type="o" direction="in"/><arg type="s" direction="in"/></method>
<method name="DisplayPasskey"><arg type="o" direction="in"/><arg type="u" direction="in"/><arg type="q" direction="in"/></method>
<method name="RequestConfirmation"><arg type="o" direction="in"/><arg type="u" direction="in"/></method>
<method name="RequestAuthorization"><arg type="o" direction="in"/></method>
<method name="AuthorizeService"><arg type="o" direction="in"/><arg type="s" direction="in"/></method>
</interface></node>"#;
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
static PAIRING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct Pairing;
impl Drop for Pairing {
    fn drop(&mut self) {
        PAIRING.store(false, std::sync::atomic::Ordering::Release);
    }
}
pub async fn pair(
    bus: &gio::DBusConnection,
    device: &str,
    name: &str,
    parent: Option<&gtk::Window>,
) -> Result<(), ErrorBody> {
    if PAIRING.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return Err(error("Finish the current pairing request first"));
    }
    let _guard = Pairing;
    let owner = bus
        .call_future(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "GetNameOwner",
            Some(&("org.bluez",).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            5000,
        )
        .await
        .map_err(error)?
        .child_value(0)
        .str()
        .unwrap_or_default()
        .to_owned();
    let info = gio::DBusNodeInfo::for_xml(XML)
        .map_err(error)?
        .lookup_interface("org.bluez.Agent1")
        .ok_or_else(|| error("Pairing interface unavailable"))?;
    let dialogs: Rc<RefCell<Vec<Prompt>>> = Rc::new(RefCell::new(vec![]));
    let active = dialogs.clone();
    let device_path = device.to_owned();
    let device_name = name.to_owned();
    let parent = parent.cloned();
    let bus_for_dialog = bus.clone();
    let registration = bus
        .register_object(PATH, &info)
        .method_call(move |_, sender, _, _, method, params, invocation| {
            if sender != Some(owner.as_str()) {
                invocation.return_dbus_error("org.bluez.Error.Rejected", "Untrusted pairing request");
                return;
            }
            if method == "Cancel" || method == "Release" {
                for dialog in active.borrow_mut().drain(..) {
                    (dialog.finish)(false);
                }
                invocation.return_value(None);
                return;
            }
            if params.try_child_value(0).and_then(|v| v.str().map(str::to_owned)).as_deref()
                != Some(device_path.as_str())
            {
                invocation.return_dbus_error("org.bluez.Error.Rejected", "Unexpected Bluetooth device");
                return;
            }
            let prompt = match method {
                "RequestConfirmation" => format!(
                    "Does {} match the code on {}?",
                    params.child_value(1).get::<u32>().map(|v| format!("{v:06}")).unwrap_or_default(),
                    device_name
                ),
                "RequestPinCode" => format!("Enter the pairing PIN for {device_name}."),
                "RequestPasskey" => format!("Enter the six-digit passkey for {device_name}."),
                "DisplayPinCode" => format!(
                    "Type {} on {} and press Enter.",
                    params.child_value(1).str().unwrap_or_default(),
                    device_name
                ),
                "DisplayPasskey" => format!(
                    "Type {:06} on {} and press Enter.",
                    params.child_value(1).get::<u32>().unwrap_or_default(),
                    device_name
                ),
                "RequestAuthorization" | "AuthorizeService" => {
                    format!("Allow {device_name} to pair with this computer?")
                }
                _ => {
                    invocation.return_dbus_error("org.bluez.Error.Rejected", "Unsupported pairing request");
                    return;
                }
            };
            active.borrow_mut().retain(|prompt| {
                if prompt.notification {
                    (prompt.finish)(true);
                    false
                } else {
                    true
                }
            });
            let window = gtk::Window::builder().title("Bluetooth pairing").modal(true).decorated(false).build();
            window.add_css_class("meridian-preferences");
            if crate::desktop::get().appearance == "dark" {
                window.add_css_class("meridian-dark");
            }
            if let Some(parent) = &parent {
                window.set_transient_for(Some(parent));
            }
            let area = gtk::Box::new(gtk::Orientation::Vertical, 14);
            area.set_margin_top(24);
            area.set_margin_bottom(24);
            area.set_margin_start(24);
            area.set_margin_end(24);
            let heading = gtk::Label::new(Some("Bluetooth pairing"));
            heading.add_css_class("title-3");
            area.append(&heading);
            let label = gtk::Label::new(Some(&prompt));
            label.set_wrap(true);
            label.set_max_width_chars(42);
            area.append(&label);
            let entry = gtk::Entry::new();
            let needs_input = method == "RequestPinCode" || method == "RequestPasskey";
            if needs_input {
                entry.set_visibility(false);
                entry.set_max_length(16);
                area.append(&entry);
            }
            let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            buttons.set_halign(gtk::Align::End);
            let cancel = gtk::Button::with_label("Cancel");
            let accept = gtk::Button::with_label("Continue");
            accept.add_css_class("suggested-action");
            buttons.append(&cancel);
            buttons.append(&accept);
            area.append(&buttons);
            window.set_child(Some(&area));
            let notification = method.starts_with("Display");
            let response = Rc::new(RefCell::new(if notification {
                invocation.return_value(None);
                None
            } else {
                Some(invocation)
            }));
            let method = method.to_owned();
            let weak = window.downgrade();
            let done = Cell::new(false);
            let pair_bus = bus_for_dialog.clone();
            let pair_device = device_path.clone();
            let finish: Rc<dyn Fn(bool)> = Rc::new(move |accepted| {
                if done.replace(true) {
                    return;
                }
                if let Some(invocation) = response.borrow_mut().take() {
                    if !accepted {
                        invocation.return_dbus_error("org.bluez.Error.Rejected", "Pairing cancelled");
                    } else if method == "RequestPinCode" && !entry.text().is_empty() {
                        invocation.return_value(Some(&(entry.text().as_str(),).to_variant()));
                    } else if method == "RequestPasskey" {
                        match entry.text().parse::<u32>() {
                            Ok(value) if value <= 999999 => invocation.return_value(Some(&(value,).to_variant())),
                            _ => invocation.return_dbus_error("org.bluez.Error.Rejected", "Invalid passkey"),
                        }
                    } else if needs_input {
                        invocation.return_dbus_error("org.bluez.Error.Rejected", "Enter a PIN");
                    } else {
                        invocation.return_value(None);
                    }
                }
                if !accepted && notification {
                    pair_bus.call(
                        Some("org.bluez"),
                        &pair_device,
                        "org.bluez.Device1",
                        "CancelPairing",
                        None,
                        None,
                        gio::DBusCallFlags::NONE,
                        5000,
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                }
                entry.set_text("");
                if let Some(window) = weak.upgrade() {
                    window.destroy();
                }
            });
            let no = finish.clone();
            cancel.connect_clicked(move |_| no(false));
            let yes = finish.clone();
            accept.connect_clicked(move |_| yes(true));
            let close = finish.clone();
            window.connect_close_request(move |_| {
                close(false);
                glib::Propagation::Stop
            });
            window.present();
            active.borrow_mut().push(Prompt { _window: window, finish, notification });
        })
        .build()
        .map_err(error)?;
    let result = bus
        .call_future(
            Some("org.bluez"),
            "/org/bluez",
            "org.bluez.AgentManager1",
            "RegisterAgent",
            Some(&(glib::variant::ObjectPath::try_from(PATH).map_err(error)?, "KeyboardDisplay").to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            10000,
        )
        .await;
    let result = match result {
        Ok(_) => bus
            .call_future(
                Some("org.bluez"),
                device,
                "org.bluez.Device1",
                "Pair",
                None,
                None,
                gio::DBusCallFlags::NONE,
                90000,
            )
            .await
            .map(|_| ())
            .map_err(error),
        Err(e) => Err(error(e)),
    };
    for dialog in dialogs.borrow_mut().drain(..) {
        (dialog.finish)(false);
    }
    let _ = bus
        .call_future(
            Some("org.bluez"),
            "/org/bluez",
            "org.bluez.AgentManager1",
            "UnregisterAgent",
            Some(&(glib::variant::ObjectPath::try_from(PATH).map_err(error)?,).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            5000,
        )
        .await;
    let _ = bus.unregister_object(registration);
    result
}
