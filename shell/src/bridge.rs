//! UI → host request handling for one WebView.
//!
//! Each WebView gets its own `UserContentManager` with a `meridian` script
//! message handler (with reply). Messages are parsed into the closed
//! `Request` enum and checked against the surface's capabilities *before*
//! they reach the dispatcher.

use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};

use gtk::glib;
use meridian_protocol::{self as proto, ErrorBody, ErrorCode, Request};
use webkit6::javascriptcore;
use webkit6::{ScriptMessageReply, UserContentManager};

use crate::surface::WebSurface;

const HANDLER: &str = "meridian";

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

pub fn content_manager() -> UserContentManager {
    let ucm = UserContentManager::new();
    // Main world: the UI is first-party code, there are no page scripts to
    // isolate from.
    ucm.register_script_message_handler_with_reply(HANDLER, None);
    ucm
}

pub fn attach(ucm: &UserContentManager, surface: Weak<WebSurface>, dispatch: Dispatch) {
    ucm.connect_script_message_with_reply_received(Some(HANDLER), move |_, value, reply| {
        let Some(ctx) = value.context() else { return true };
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
                glib::MainContext::default().spawn_local(async move {
                    let result = future.await;
                    send(&reply, &ctx, result);
                });
            }
        }
        true
    });
}

fn send(reply: &ScriptMessageReply, ctx: &javascriptcore::Context, result: ReplyResult) {
    let json = result.unwrap_or_else(|e| {
        log::warn!("request failed: {:?}: {}", e.code, e.message);
        proto::reply_err(&e)
    });
    reply.return_value(&javascriptcore::Value::new_string(ctx, Some(&json)));
}

fn handle(surface: &Rc<WebSurface>, value: &javascriptcore::Value, dispatch: &Dispatch) -> Result<Reply, ErrorBody> {
    if !value.is_string() {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "message must be a JSON string"));
    }
    let request = proto::parse_request(&value.to_str())?;
    authorize(surface, &request)?;
    log::trace!("{:?} → {request:?}", surface.kind);
    dispatch(surface, request)
}

fn authorize(surface: &WebSurface, request: &Request) -> Result<(), ErrorBody> {
    match request.required_capability() {
        Some(cap) if !surface.kind.has(cap) => Err(ErrorBody::new(
            ErrorCode::PermissionDenied,
            format!("{:?} surface lacks the {cap:?} capability", surface.kind),
        )),
        _ => Ok(()),
    }
}
