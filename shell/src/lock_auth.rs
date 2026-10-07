//! Password verification for the lock screen, via PAM (the `meridian-lock`
//! service; see `session/meridian-lock.pam`). Runs on gio's blocking thread
//! pool so a slow PAM module (e.g. a network directory) can't freeze the UI.
//!
//! This only ever checks the password of the user already running this
//! session (never an arbitrary username), which is what lets an
//! unprivileged process do it: PAM's `pam_unix` goes through the
//! `unix_chkpwd` helper for that case on every distribution Meridian targets.

use gtk::gio;
use meridian_protocol::{ErrorBody, ErrorCode};
use pam_client::conv_mock::Conversation;
use pam_client::{Context, Flag};

const SERVICE: &str = "meridian-lock";

pub fn check_setup() -> Result<(), ErrorBody> {
    std::fs::read_to_string("/etc/pam.d/meridian-lock").map(|_| ()).map_err(|e| {
        log::warn!("lock screen: cannot read PAM service: {e}");
        ErrorBody::new(
            ErrorCode::Unavailable,
            "Lock screen authentication is not configured. Install /etc/pam.d/meridian-lock.",
        )
    })
}

pub async fn verify(username: String, password: String) -> Result<(), ErrorBody> {
    match gio::spawn_blocking(move || verify_blocking(&username, &password)).await {
        Ok(result) => result,
        Err(_) => Err(ErrorBody::new(ErrorCode::Failed, "authentication crashed")),
    }
}

fn verify_blocking(username: &str, password: &str) -> Result<(), ErrorBody> {
    check_setup()?;
    let conversation = Conversation::with_credentials(username, password);
    let mut context = Context::new(SERVICE, Some(username), conversation)
        .map_err(|e| ErrorBody::new(ErrorCode::Unavailable, format!("PAM: {e}")))?;
    context.authenticate(Flag::NONE).map_err(auth_failed)?;
    context.acct_mgmt(Flag::NONE).map_err(|e| {
        log::warn!("lock screen: account check failed: {e}");
        ErrorBody::new(
            ErrorCode::PermissionDenied,
            "Your account cannot be unlocked. Check the system authentication logs.",
        )
    })?;
    Ok(())
}

fn auth_failed(e: pam_client::Error) -> ErrorBody {
    log::warn!("lock screen: authentication failed: {e}");
    match e.code() {
        pam_client::ErrorCode::AUTH_ERR => ErrorBody::new(ErrorCode::AuthFailed, "Incorrect password"),
        pam_client::ErrorCode::MAXTRIES => ErrorBody::new(ErrorCode::AuthFailed, "Too many attempts. Try again later."),
        _ => ErrorBody::new(
            ErrorCode::Unavailable,
            "Authentication is unavailable. Check the system authentication logs.",
        ),
    }
}
