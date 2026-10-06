//! Settings shown in the Options menu: Wi-Fi, Bluetooth, brightness, volume.
//!
//! Each control talks to the system service that owns it; privileged
//! changes are authorized by those services (polkit), never by the shell.

mod audio;
mod backlight;
mod dbus_bool;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use meridian_protocol::{ErrorBody, ErrorCode, SettingsState};

use self::audio::Audio;
use self::backlight::Backlight;
use self::dbus_bool::BoolProperty;

const NM_NAME: &str = "org.freedesktop.NetworkManager";
const BLUEZ_NAME: &str = "org.bluez";

type Listener = Box<dyn Fn(&SettingsState)>;

#[derive(Default)]
struct Inner {
    wifi: RefCell<Option<BoolProperty>>,
    bluetooth: RefCell<Option<BoolProperty>>,
    backlight: Option<Backlight>,
    audio: RefCell<Option<Audio>>,
    /// Last known volume (read asynchronously).
    volume: RefCell<Option<u8>>,
    last_sent: RefCell<Option<SettingsState>>,
    listeners: RefCell<Vec<Listener>>,
}

/// Cheap to clone; all clones share state.
#[derive(Clone)]
pub struct SettingsService {
    inner: Rc<Inner>,
}

impl SettingsService {
    pub fn new() -> Self {
        let inner = Rc::new(Inner { backlight: Backlight::find(), ..Default::default() });
        let service = SettingsService { inner };

        let weak = Rc::downgrade(&service.inner);
        *service.inner.wifi.borrow_mut() = Some(BoolProperty::watch(
            NM_NAME,
            "/org/freedesktop/NetworkManager".into(),
            NM_NAME,
            "WirelessEnabled",
            notifier(&weak),
        ));

        let weak = Rc::downgrade(&service.inner);
        *service.inner.audio.borrow_mut() = Some(Audio::new(move |message| {
            if let Some(inner) = weak.upgrade() {
                SettingsService { inner }.report(&message);
            }
        }));

        service.find_bluetooth_adapter();
        service.refresh();
        service
    }

    /// BlueZ adapters live at dynamic paths; use the first one found.
    fn find_bluetooth_adapter(&self) {
        let Ok(bus) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) else { return };
        let weak = Rc::downgrade(&self.inner);
        bus.call(
            Some(BLUEZ_NAME),
            "/",
            "org.freedesktop.DBus.ObjectManager",
            "GetManagedObjects",
            None,
            None,
            gio::DBusCallFlags::NO_AUTO_START,
            -1,
            gio::Cancellable::NONE,
            move |result| {
                let Some(inner) = weak.upgrade() else { return };
                let objects = match result {
                    Ok(v) => v,
                    Err(e) => return log::info!("bluetooth unavailable: {e}"),
                };
                let Some(path) = first_adapter(&objects) else {
                    return log::info!("bluetooth: no adapter");
                };
                log::info!("bluetooth adapter: {path}");
                let watch = BoolProperty::watch(
                    BLUEZ_NAME,
                    path,
                    "org.bluez.Adapter1",
                    "Powered",
                    notifier(&Rc::downgrade(&inner)),
                );
                *inner.bluetooth.borrow_mut() = Some(watch);
            },
        );
    }

    pub fn state(&self) -> SettingsState {
        let inner = &self.inner;
        SettingsState {
            wifi: inner.wifi.borrow().as_ref().and_then(BoolProperty::get),
            bluetooth: inner.bluetooth.borrow().as_ref().and_then(BoolProperty::get),
            brightness: inner.backlight.as_ref().and_then(Backlight::percent),
            volume: *inner.volume.borrow(),
        }
    }

    /// Re-read values that have no change notifications (brightness,
    /// volume). Call when the Options menu opens.
    pub fn refresh(&self) {
        self.notify();
        let weak = Rc::downgrade(&self.inner);
        if let Some(audio) = self.inner.audio.borrow().as_ref() {
            audio.read(move |volume| {
                if let Some(inner) = weak.upgrade() {
                    *inner.volume.borrow_mut() = volume;
                    SettingsService { inner }.notify();
                }
            });
        }
    }

    pub fn set_wifi(&self, enabled: bool) -> Result<(), ErrorBody> {
        self.set_bool(&self.inner.wifi, enabled, "Wi-Fi")
    }

    pub fn set_bluetooth(&self, enabled: bool) -> Result<(), ErrorBody> {
        self.set_bool(&self.inner.bluetooth, enabled, "Bluetooth")
    }

    fn set_bool(&self, prop: &RefCell<Option<BoolProperty>>, value: bool, what: &'static str) -> Result<(), ErrorBody> {
        let prop = prop.borrow();
        let prop = prop.as_ref().filter(|p| p.get().is_some()).ok_or_else(|| unavailable(what))?;
        let weak = Rc::downgrade(&self.inner);
        prop.set(value, move |e| {
            if let Some(inner) = weak.upgrade() {
                let service = SettingsService { inner };
                service.report(&format!("{what}: {}", e.message()));
                // Re-send the real state so the toggle snaps back.
                service.force_notify();
            }
        });
        Ok(())
    }

    pub fn set_brightness(&self, percent: u8) -> Result<(), ErrorBody> {
        let backlight = self.inner.backlight.as_ref().ok_or_else(|| unavailable("Brightness"))?;
        let weak = Rc::downgrade(&self.inner);
        backlight.set_percent(percent, move |e| {
            if let Some(inner) = weak.upgrade() {
                let service = SettingsService { inner };
                service.report(&format!("Brightness: {}", e.message()));
                service.force_notify();
            }
        });
        Ok(())
    }

    pub fn set_volume(&self, percent: u8) -> Result<(), ErrorBody> {
        let audio = self.inner.audio.borrow();
        let audio =
            audio.as_ref().filter(|_| self.inner.volume.borrow().is_some()).ok_or_else(|| unavailable("Volume"))?;
        *self.inner.volume.borrow_mut() = Some(percent);
        audio.set(percent);
        Ok(())
    }

    /// Called with the new state whenever it changes.
    pub fn connect_changed(&self, f: impl Fn(&SettingsState) + 'static) {
        self.inner.listeners.borrow_mut().push(Box::new(f));
    }

    fn notify(&self) {
        let state = self.state();
        if self.inner.last_sent.borrow().as_ref() == Some(&state) {
            return;
        }
        *self.inner.last_sent.borrow_mut() = Some(state.clone());
        for l in self.inner.listeners.borrow().iter() {
            l(&state);
        }
    }

    fn force_notify(&self) {
        self.inner.last_sent.borrow_mut().take();
        self.notify();
    }

    fn report(&self, message: &str) {
        log::warn!("{message}");
    }
}

impl Default for SettingsService {
    fn default() -> Self {
        Self::new()
    }
}

fn notifier(weak: &Weak<Inner>) -> impl Fn() + 'static {
    let weak = weak.clone();
    move || {
        if let Some(inner) = weak.upgrade() {
            SettingsService { inner }.notify();
        }
    }
}

fn unavailable(what: &str) -> ErrorBody {
    ErrorBody::new(ErrorCode::Unavailable, format!("{what} is not available"))
}

/// First object implementing `org.bluez.Adapter1` in a
/// `GetManagedObjects` reply of type `(a{oa{sa{sv}}})`.
fn first_adapter(reply: &glib::Variant) -> Option<String> {
    let objects = reply.try_child_value(0)?;
    let mut paths: Vec<String> = (0..objects.n_children())
        .filter_map(|i| {
            let entry = objects.child_value(i);
            let path = entry.child_value(0).str()?.to_owned();
            let interfaces = entry.child_value(1);
            let has_adapter = (0..interfaces.n_children())
                .any(|j| interfaces.child_value(j).child_value(0).str() == Some("org.bluez.Adapter1"));
            has_adapter.then_some(path)
        })
        .collect();
    paths.sort();
    paths.into_iter().next()
}
