//! Client for greetd's IPC protocol (`greetd-ipc(7)`).
//!
//! Messages are JSON, each prefixed by its length as a native-endian u32,
//! over the Unix socket named by `$GREETD_SOCK`. greetd runs PAM itself;
//! the greeter only relays prompts and answers. I/O is asynchronous on the
//! GLib main loop, so the login screen stays responsive during PAM delays.

use gio::prelude::*;
use gtk::{gio, glib};
use serde::{Deserialize, Serialize};

/// Largest response we accept; greetd's messages are tiny.
const MAX_MESSAGE: u32 = 64 * 1024;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request<'a> {
    CreateSession { username: &'a str },
    PostAuthMessageResponse { response: Option<&'a str> },
    StartSession { cmd: &'a [String], env: &'a [String] },
    CancelSession,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Success,
    Error { error_type: ErrorType, description: String },
    AuthMessage { auth_message_type: AuthMessageType, auth_message: String },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ErrorType {
    AuthError,
    Error,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum AuthMessageType {
    Visible,
    Secret,
    Info,
    Error,
}

#[derive(Debug, PartialEq)]
pub enum LoginError {
    /// Wrong username or password.
    Auth,
    /// The account needs a login step this greeter can't do (e.g. a
    /// second factor).
    Unsupported(String),
    /// greetd unavailable or protocol failure.
    Failed(String),
}

impl From<glib::Error> for LoginError {
    fn from(e: glib::Error) -> Self {
        LoginError::Failed(e.message().to_owned())
    }
}

pub struct Client {
    conn: gio::SocketConnection,
}

impl Client {
    /// Connect to the greetd instance that started this greeter.
    pub async fn connect() -> Result<Client, LoginError> {
        let path = std::env::var("GREETD_SOCK")
            .map_err(|_| LoginError::Failed("GREETD_SOCK is not set (not started by greetd?)".into()))?;
        let address = gio::UnixSocketAddress::new(std::path::Path::new(&path));
        let conn = gio::SocketClient::new().connect_future(&address).await?;
        Ok(Client { conn })
    }

    async fn send(&self, request: &Request<'_>) -> Result<(), LoginError> {
        let body = serde_json::to_vec(request).map_err(|e| LoginError::Failed(e.to_string()))?;
        let mut message = (body.len() as u32).to_ne_bytes().to_vec();
        message.extend_from_slice(&body);
        self.conn
            .output_stream()
            .write_all_future(message, glib::Priority::DEFAULT)
            .await
            .map_err(|(_, e)| LoginError::from(e))?;
        Ok(())
    }

    async fn receive(&self) -> Result<Response, LoginError> {
        let input = self.conn.input_stream();
        let (header, n, _) =
            input.read_all_future(vec![0u8; 4], glib::Priority::DEFAULT).await.map_err(|(_, e)| LoginError::from(e))?;
        if n != 4 {
            return Err(LoginError::Failed("greetd closed the connection".into()));
        }
        let len = u32::from_ne_bytes([header[0], header[1], header[2], header[3]]);
        if len > MAX_MESSAGE {
            return Err(LoginError::Failed(format!("oversized greetd message ({len} bytes)")));
        }
        let (body, n, _) = input
            .read_all_future(vec![0u8; len as usize], glib::Priority::DEFAULT)
            .await
            .map_err(|(_, e)| LoginError::from(e))?;
        if n != len as usize {
            return Err(LoginError::Failed("truncated greetd message".into()));
        }
        serde_json::from_slice(&body).map_err(|e| LoginError::Failed(format!("bad greetd message: {e}")))
    }

    /// Authenticate `username` with `password`, then ask greetd to start
    /// `cmd` with `env` once this greeter exits.
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        cmd: &[String],
        env: &[String],
    ) -> Result<(), LoginError> {
        self.send(&Request::CreateSession { username }).await?;
        let mut answered_password = false;
        loop {
            let response = self.receive().await?;
            match next_step(&response, answered_password) {
                Step::Done => break,
                Step::Answer(Answer::Password) => {
                    answered_password = true;
                    self.send(&Request::PostAuthMessageResponse { response: Some(password) }).await?;
                }
                Step::Answer(Answer::Username) => {
                    self.send(&Request::PostAuthMessageResponse { response: Some(username) }).await?;
                }
                Step::Answer(Answer::Acknowledge) => {
                    self.send(&Request::PostAuthMessageResponse { response: None }).await?;
                }
                Step::Fail(error) => {
                    // Reset greetd so the next attempt starts cleanly.
                    let _ = self.cancel().await;
                    return Err(error);
                }
            }
        }
        self.send(&Request::StartSession { cmd, env }).await?;
        match self.receive().await? {
            Response::Success => Ok(()),
            Response::Error { description, .. } => {
                let _ = self.cancel().await;
                Err(LoginError::Failed(description))
            }
            other => Err(LoginError::Failed(format!("unexpected greetd reply: {other:?}"))),
        }
    }

    async fn cancel(&self) -> Result<(), LoginError> {
        self.send(&Request::CancelSession).await?;
        self.receive().await.map(|_| ())
    }
}

enum Answer {
    Password,
    Username,
    Acknowledge,
}

enum Step {
    Done,
    Answer(Answer),
    Fail(LoginError),
}

/// The authentication conversation, as a pure function for testing.
fn next_step(response: &Response, answered_password: bool) -> Step {
    match response {
        Response::Success => Step::Done,
        Response::Error { error_type: ErrorType::AuthError, .. } => Step::Fail(LoginError::Auth),
        Response::Error { description, .. } => Step::Fail(LoginError::Failed(description.clone())),
        Response::AuthMessage { auth_message_type: AuthMessageType::Secret, auth_message } => {
            if answered_password {
                Step::Fail(LoginError::Unsupported(auth_message.clone()))
            } else {
                Step::Answer(Answer::Password)
            }
        }
        // Some PAM stacks ask for the login name again as a visible prompt.
        Response::AuthMessage { auth_message_type: AuthMessageType::Visible, .. } => Step::Answer(Answer::Username),
        Response::AuthMessage { .. } => Step::Answer(Answer::Acknowledge),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_format_matches_greetd_ipc() {
        let json = serde_json::to_string(&Request::CreateSession { username: "sai" }).unwrap();
        assert_eq!(json, r#"{"type":"create_session","username":"sai"}"#);
        let json = serde_json::to_string(&Request::PostAuthMessageResponse { response: None }).unwrap();
        assert_eq!(json, r#"{"type":"post_auth_message_response","response":null}"#);
        let r: Response =
            serde_json::from_str(r#"{"type":"auth_message","auth_message_type":"secret","auth_message":"Password: "}"#)
                .unwrap();
        assert_eq!(
            r,
            Response::AuthMessage { auth_message_type: AuthMessageType::Secret, auth_message: "Password: ".into() }
        );
        let r: Response =
            serde_json::from_str(r#"{"type":"error","error_type":"auth_error","description":"pam"}"#).unwrap();
        assert!(matches!(next_step(&r, true), Step::Fail(LoginError::Auth)));
    }

    #[test]
    fn conversation() {
        let secret =
            Response::AuthMessage { auth_message_type: AuthMessageType::Secret, auth_message: "Password:".into() };
        assert!(matches!(next_step(&secret, false), Step::Answer(Answer::Password)));
        // A second secret prompt (e.g. an OTP) isn't supported.
        assert!(matches!(next_step(&secret, true), Step::Fail(LoginError::Unsupported(_))));
        let info = Response::AuthMessage { auth_message_type: AuthMessageType::Info, auth_message: "hi".into() };
        assert!(matches!(next_step(&info, true), Step::Answer(Answer::Acknowledge)));
        assert!(matches!(next_step(&Response::Success, true), Step::Done));
    }
}
