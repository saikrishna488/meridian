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

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
struct AuthState {
    busy: bool,
    retry_at: Option<Instant>,
}
static AUTH: Mutex<AuthState> = Mutex::new(AuthState { busy: false, retry_at: None });
struct Attempt {
    accepted: bool,
}
impl Attempt {
    fn start() -> Result<Self, ErrorBody> {
        let mut state = auth_state()?;
        if state.busy || state.retry_at.is_some_and(|time| time > Instant::now()) {
            return Err(ErrorBody::new(ErrorCode::Unavailable, "Authentication is busy. Wait a moment and try again."));
        }
        state.busy = true;
        Ok(Self { accepted: false })
    }
}
fn auth_state() -> Result<MutexGuard<'static, AuthState>, ErrorBody> {
    AUTH.lock().map_err(|_| ErrorBody::new(ErrorCode::Unavailable, "Authentication is unavailable."))
}
impl Drop for Attempt {
    fn drop(&mut self) {
        if let Ok(mut state) = AUTH.lock() {
            state.busy = false;
            state.retry_at = (!self.accepted).then(|| Instant::now() + Duration::from_secs(1));
        }
    }
}

const SERVICE: &str = "meridian-lock";

pub fn check_setup() -> Result<(), ErrorBody> {
    use std::os::unix::fs::MetadataExt;
    let path = std::path::Path::new("/etc/pam.d/meridian-lock");
    let metadata = std::fs::metadata(path).map_err(|_| {
        ErrorBody::new(
            ErrorCode::Unavailable,
            "Lock screen authentication is not configured. Install /etc/pam.d/meridian-lock.",
        )
    })?;
    if !metadata.is_file() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return Err(ErrorBody::new(
            ErrorCode::Unavailable,
            "The lock PAM service must be a root-owned file without group or other write permission.",
        ));
    }
    std::fs::read_to_string(path)
        .map(|_| ())
        .map_err(|_| ErrorBody::new(ErrorCode::Unavailable, "The lock PAM service cannot be read."))
}

pub async fn verify(username: String, password: String) -> Result<(), ErrorBody> {
    if password.len() > 4096 || password.contains('\0') {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Invalid password input."));
    }
    let mut attempt = Attempt::start()?;
    match gio::spawn_blocking(move || {
        let result = verify_blocking(&username, &password);
        attempt.accepted = result.is_ok();
        drop(attempt);
        result
    })
    .await
    {
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
        log::warn!("component=authentication event=account_rejected code={:?}", e.code());
        ErrorBody::new(
            ErrorCode::PermissionDenied,
            "Your account cannot be unlocked. Check the system authentication logs.",
        )
    })?;
    Ok(())
}

fn auth_failed(e: pam_client::Error) -> ErrorBody {
    log::warn!("component=authentication event=password_rejected code={:?}", e.code());
    match e.code() {
        pam_client::ErrorCode::AUTH_ERR => ErrorBody::new(ErrorCode::AuthFailed, "Incorrect password"),
        pam_client::ErrorCode::MAXTRIES => ErrorBody::new(ErrorCode::AuthFailed, "Too many attempts. Try again later."),
        _ => ErrorBody::new(
            ErrorCode::Unavailable,
            "Authentication is unavailable. Check the system authentication logs.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentication_is_serialized_across_monitors_and_backed_off() {
        let mut first = Attempt::start().unwrap();
        assert!(Attempt::start().is_err());
        first.accepted = true;
        drop(first);
        let second = Attempt::start().unwrap();
        drop(second);
        assert!(Attempt::start().is_err());
        let mut state = AUTH.lock().unwrap();
        state.retry_at = None;
        assert!(!state.busy);
    }
}
