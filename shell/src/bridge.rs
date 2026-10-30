//! UI → host request handling for one WebView.
//!
//! Each WebView gets its own `UserContentManager` with a `meridian` script
//! message handler (with reply). Messages are parsed into the closed
//! `Request` enum and checked against the surface's capabilities *before*
//! they reach the dispatcher.

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};

use gtk::glib;
use meridian_protocol::{self as proto, ErrorBody, ErrorCode, Request};
use webkit6::javascriptcore;
use webkit6::{ScriptMessageReply, UserContentManager};

use crate::surface::WebSurface;

const HANDLER: &str = "meridian";
const MAX_PENDING: usize = 16;
thread_local! {
    static PENDING: Cell<usize> = const { Cell::new(0) };
}

struct Pending;
impl Drop for Pending {
    fn drop(&mut self) {
        PENDING.with(|pending| pending.set(pending.get().saturating_sub(1)));
    }
}

/// A serialized reply payload (`reply_ok(...)`), or an error.
pub type ReplyResult = Result<String, ErrorBody>;

/// What a dispatcher returns: a reply now, or one computed asynchronously
/// on the main loop (e.g. authentication).
pub enum Reply {
    Now(String),
    Later(Pin<Box<dyn Future<Output = ReplyResult>>>),
}

/// Executes an already-authorized request for a surface.
pub type Dispatch = Rc<dyn Fn(&Rc<WebSurface>, Request) -> Result<Reply, ErrorBody>>;

#[cfg(any(test, feature = "integration-tests"))]
thread_local! { static REQUEST_COUNTS: std::cell::RefCell<std::collections::BTreeMap<String, u64>> = Default::default(); }
#[cfg(any(test, feature = "integration-tests"))]
pub(crate) fn request_counts() -> std::collections::BTreeMap<String, u64> {
    REQUEST_COUNTS.with(|counts| counts.borrow().clone())
}
pub fn content_manager() -> UserContentManager {
    let ucm = UserContentManager::new();
    // Main world: the UI is first-party code, there are no page scripts to
    // isolate from.
    ucm.register_script_message_handler_with_reply(HANDLER, None);
    ucm
}

pub fn attach(ucm: &UserContentManager, surface: Weak<WebSurface>, dispatch: Dispatch) {
    let pending = Rc::new(Cell::new(0usize));
    let budget = Rc::new(std::cell::RefCell::new((std::time::Instant::now(), 0u32)));
    ucm.connect_script_message_with_reply_received(Some(HANDLER), move |_, value, reply| {
        let Some(ctx) = value.context() else { return true };
        let limited = {
            let mut budget = budget.borrow_mut();
            if budget.0.elapsed() >= std::time::Duration::from_secs(1) {
                *budget = (std::time::Instant::now(), 0);
            }
            budget.1 = budget.1.saturating_add(1);
            budget.1 > 256 || pending.get() >= MAX_PENDING || PENDING.with(|n| n.get() >= 128)
        };
        if limited {
            send(reply, &ctx, Err(ErrorBody::new(ErrorCode::Unavailable, "Too many requests; try again shortly.")));
            return true;
        }
        let result = match surface.upgrade() {
            Some(surface) => handle(&surface, value, &dispatch),
            None => Err(ErrorBody::new(ErrorCode::Unavailable, "surface destroyed")),
        };
        match result {
            Ok(Reply::Now(json)) => send(reply, &ctx, Ok(json)),
            Err(e) => send(reply, &ctx, Err(e)),
            Ok(Reply::Later(future)) => {
                // The reply object stays alive (and the JS promise pending)
                // until we answer.
                let reply = reply.clone();
                pending.set(pending.get() + 1);
                let pending = pending.clone();
                PENDING.with(|n| n.set(n.get() + 1));
                glib::MainContext::default().spawn_local(async move {
                    let _guard = Pending;
                    let result = future.await;
                    pending.set(pending.get().saturating_sub(1));
                    send(&reply, &ctx, result);
                });
            }
        }
        true
    });
}

fn send(reply: &ScriptMessageReply, ctx: &javascriptcore::Context, result: ReplyResult) {
    let json = result.unwrap_or_else(|e| {
        // Error messages from system services may contain user input.
        log::warn!("component=bridge event=request_failed code={:?}", e.code);
        proto::reply_err(&e)
    });
    reply.return_value(&javascriptcore::Value::new_string(ctx, Some(&json)));
}

fn handle(surface: &Rc<WebSurface>, value: &javascriptcore::Value, dispatch: &Dispatch) -> Result<Reply, ErrorBody> {
    if !value.is_string() {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "message must be a JSON string"));
    }
    let request = proto::parse_request(&value.to_str())?;
    authorize_kind(surface.kind, &request)?;
    #[cfg(any(test, feature = "integration-tests"))]
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&value.to_str())
        && let Some(method) = value["method"].as_str()
    {
        REQUEST_COUNTS
            .with(|counts| *counts.borrow_mut().entry(format!("{:?}/{method}", surface.kind)).or_default() += 1);
    }
    log::trace!("component=bridge surface={:?} capability={:?}", surface.kind, request.required_capability());
    dispatch(surface, request)
}

fn authorize_kind(surface: meridian_protocol::SurfaceKind, request: &Request) -> Result<(), ErrorBody> {
    match request.required_capability() {
        Some(cap) if !surface.has(cap) => Err(ErrorBody::new(
            ErrorCode::PermissionDenied,
            format!("{surface:?} surface lacks the {cap:?} capability"),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_protocol::SurfaceKind::*;
    #[test]
    fn sensitive_requests_are_denied_before_dispatch() {
        let surfaces = [
            Wallpaper,
            Panel,
            Topbar,
            Preferences,
            Finder,
            Terminal,
            Notifications,
            Pulse,
            Calendar,
            Clock,
            DesktopMenu,
            Applications,
            Options,
            Greeter,
            Locker,
        ];
        for (json, allowed) in [
            (r#"{"method":"terminal.start"}"#, vec![Terminal]),
            (r#"{"method":"terminal.write","params":{"id":1,"data":"touch /tmp/never-executed"}}"#, vec![Terminal]),
            (r#"{"method":"terminal.paste"}"#, vec![Terminal]),
            (r#"{"method":"settings.users"}"#, vec![Preferences]),
            (r#"{"method":"desktop.add_file","params":{"path":"/tmp/a"}}"#, vec![Wallpaper, Finder]),
            (r#"{"method":"desktop.remove","params":{"id":"file:/tmp/a"}}"#, vec![Wallpaper, Finder]),
            (r#"{"method":"settings.drivers"}"#, vec![Preferences]),
            (
                r#"{"method":"files.transfer_start","params":{"path":"/tmp/a","destination":"/tmp","move_file":true}}"#,
                vec![Finder],
            ),
            (r#"{"method":"files.transfer_cancel","params":{"id":1}}"#, vec![Finder]),
            (r#"{"method":"files.trash","params":{"path":"/tmp/a"}}"#, vec![Finder]),
            (
                r#"{"method":"greeter.login","params":{"username":"test","password":"secret","session":"meridian.desktop"}}"#,
                vec![Greeter],
            ),
            (r#"{"method":"locker.unlock","params":{"password":"secret"}}"#, vec![Locker]),
        ] {
            let request = proto::parse_request(json).unwrap();
            for surface in surfaces {
                assert_eq!(authorize_kind(surface, &request).is_ok(), allowed.contains(&surface), "{surface:?} {json}");
            }
        }
    }
}
