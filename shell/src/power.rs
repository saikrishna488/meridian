//! Session-ending actions via systemd-logind: power off, reboot, suspend,
//! and ending the calling process's own session. Shared by the greeter
//! (pre-login) and the shell (the running session).

use gtk::prelude::*;
use gtk::{gio, glib};
use meridian_protocol::{ErrorBody, ErrorCode};

const LOGIND: &str = "org.freedesktop.login1";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER_IFACE: &str = "org.freedesktop.login1.Manager";
const SESSION_IFACE: &str = "org.freedesktop.login1.Session";

/// Power off the machine. `interactive = false`: logind's policy decides
/// (normally allowed for the active session).
pub async fn power_off() -> Result<(), ErrorBody> {
    manager_call("PowerOff", Some(&(false,).to_variant())).await.map(drop)
}

pub async fn reboot() -> Result<(), ErrorBody> {
    manager_call("Reboot", Some(&(false,).to_variant())).await.map(drop)
}

pub async fn suspend() -> Result<(), ErrorBody> {
    manager_call("Suspend", Some(&(false,).to_variant())).await.map(drop)
}

/// End the calling process's own session: logind kills it and its
/// children, which returns the display to the login screen.
pub async fn sign_out() -> Result<(), ErrorBody> {
    let reply = manager_call("GetSessionByPID", Some(&(std::process::id(),).to_variant())).await?;
    let path = reply
        .child_value(0)
        .str()
        .ok_or_else(|| ErrorBody::new(ErrorCode::Failed, "logind didn't report a session for this process"))?
        .to_owned();
    let bus = gio::bus_get_future(gio::BusType::System).await.map_err(unavailable)?;
    bus.call_future(Some(LOGIND), &path, SESSION_IFACE, "Terminate", None, None, gio::DBusCallFlags::NONE, -1)
        .await
        .map_err(failed)?;
    Ok(())
}

async fn manager_call(method: &str, params: Option<&glib::Variant>) -> Result<glib::Variant, ErrorBody> {
    let bus = gio::bus_get_future(gio::BusType::System).await.map_err(unavailable)?;
    bus.call_future(Some(LOGIND), MANAGER_PATH, MANAGER_IFACE, method, params, None, gio::DBusCallFlags::NONE, -1)
        .await
        .map_err(failed)
}

fn unavailable(e: glib::Error) -> ErrorBody {
    ErrorBody::new(ErrorCode::Unavailable, e.message().to_owned())
}

fn failed(e: glib::Error) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.message().to_owned())
}
