//! The login screen: one full-screen surface, run by greetd inside a
//! minimal labwc (see `session/greeter/`).
//!
//! The web UI collects the username, password, and session. Authentication
//! is done by greetd (PAM); this process only relays the answers over
//! greetd's socket and never stores the password.

use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gio, glib};
use meridian_protocol::{
    self as proto, ErrorBody, ErrorCode, GreeterInfo, GreeterLogin, PowerAction, Request, SessionInfo, SurfaceKind,
};
use serde::{Deserialize, Serialize};

use crate::bridge::{Dispatch, Reply, ReplyResult};
use crate::greetd::{self, LoginError};
use crate::scheme::{Roots, SchemeHandler};
use crate::surface::{WebEnv, WebSurface};

/// Session directories, highest precedence first.
const SESSION_DIRS: [&str; 2] = ["/usr/local/share/wayland-sessions", "/usr/share/wayland-sessions"];

/// Preferred default session on first use.
const DEFAULT_SESSION: &str = "meridian.desktop";

#[derive(Debug, Clone)]
struct Session {
    info: SessionInfo,
    cmd: Vec<String>,
    desktop_names: Option<String>,
}

/// Remembered between logins (in the greeter user's cache directory).
#[derive(Debug, Default, Serialize, Deserialize)]
struct LastLogin {
    user: Option<String>,
    session: Option<String>,
}

pub struct Greeter {
    app: gtk::Application,
    env: WebEnv,
    sessions: Vec<Session>,
    state_file: PathBuf,
    weak_self: Weak<Greeter>,
    surface: std::cell::RefCell<Option<Rc<WebSurface>>>,
    _hold: gio::ApplicationHoldGuard,
}

impl Greeter {
    pub fn start(app: &gtk::Application, roots: Roots, dev: bool) -> Rc<Greeter> {
        let sessions = load_sessions();
        log::info!("sessions: {}", sessions.iter().map(|s| s.info.id.as_str()).collect::<Vec<_>>().join(", "));
        let greeter = Rc::new_cyclic(|weak_self| Greeter {
            app: app.clone(),
            env: WebEnv::new(SchemeHandler::new(roots), dev),
            sessions,
            state_file: glib::user_cache_dir().join("meridian-greeter").join("last-login.json"),
            weak_self: weak_self.clone(),
            surface: Default::default(),
            _hold: app.hold(),
        });
        let surface = WebSurface::new(app, &greeter.env, SurfaceKind::Greeter, None, greeter.dispatcher());
        surface.window.present();
        *greeter.surface.borrow_mut() = Some(surface);
        greeter
    }

    fn dispatcher(&self) -> Dispatch {
        let weak = self.weak_self.clone();
        Rc::new(move |_surface, request| match weak.upgrade() {
            Some(greeter) => greeter.dispatch(request),
            None => Err(ErrorBody::new(ErrorCode::Unavailable, "greeter is shutting down")),
        })
    }

    fn dispatch(&self, request: Request) -> Result<Reply, ErrorBody> {
        let serialize = |r: Result<String, serde_json::Error>| {
            r.map(Reply::Now).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
        };
        match request {
            Request::Hello => {
                if let Some(surface) = self.surface.borrow().as_ref() {
                    surface.mark_ready();
                }
                serialize(proto::reply_ok(&proto::SurfaceInfo {
                    surface: SurfaceKind::Greeter,
                    capabilities: SurfaceKind::Greeter.capabilities().to_vec(),
                    dev: false,
                }))
            }
            Request::GreeterInfo => serialize(proto::reply_ok(&self.info())),
            Request::GreeterLogin(login) => {
                let greeter = self.weak_self.upgrade().expect("dispatching on a live greeter");
                Ok(Reply::Later(Box::pin(async move { greeter.login(login).await })))
            }
            Request::GreeterPower(p) => {
                let greeter = self.weak_self.upgrade().expect("dispatching on a live greeter");
                Ok(Reply::Later(Box::pin(async move { greeter.power(p.action).await })))
            }
            // Capability-gated: only the greeter's own requests reach here.
            _ => Err(ErrorBody::new(ErrorCode::InvalidRequest, "not available on the login screen")),
        }
    }

    fn info(&self) -> GreeterInfo {
        let last = self.read_last();
        let last_session = last
            .session
            .filter(|id| self.sessions.iter().any(|s| &s.info.id == id))
            .or_else(|| self.sessions.iter().find(|s| s.info.id == DEFAULT_SESSION).map(|s| s.info.id.clone()));
        GreeterInfo {
            hostname: glib::host_name().to_string(),
            sessions: self.sessions.iter().map(|s| s.info.clone()).collect(),
            last_user: last.user,
            last_session,
        }
    }

    async fn login(self: Rc<Self>, login: GreeterLogin) -> ReplyResult {
        let session = self
            .sessions
            .iter()
            .find(|s| s.info.id == login.session)
            .ok_or_else(|| ErrorBody::new(ErrorCode::NotFound, "no such session"))?;
        if login.username.trim().is_empty() {
            return Err(ErrorBody::new(ErrorCode::InvalidRequest, "enter a username"));
        }
        let env = session_env(session);
        let client = greetd::Client::connect().await.map_err(login_error)?;
        client.login(login.username.trim(), &login.password, &session.cmd, &env).await.map_err(login_error)?;
        log::info!("login accepted for {}; starting {}", login.username.trim(), session.info.id);
        self.write_last(&LastLogin {
            user: Some(login.username.trim().to_owned()),
            session: Some(session.info.id.clone()),
        });

        // greetd starts the session once the greeter exits. Leave a moment
        // for the reply to reach the page (it shows "Starting…").
        let app = self.app.clone();
        glib::timeout_add_local_once(Duration::from_millis(300), move || app.quit());
        proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
    }

    async fn power(self: Rc<Self>, action: PowerAction) -> ReplyResult {
        match action {
            PowerAction::PowerOff => crate::power::power_off().await,
            PowerAction::Reboot => crate::power::reboot().await,
        }?;
        proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
    }

    fn read_last(&self) -> LastLogin {
        std::fs::read(&self.state_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn write_last(&self, last: &LastLogin) {
        let result = (|| -> std::io::Result<()> {
            if let Some(dir) = self.state_file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&self.state_file, serde_json::to_vec(last)?)
        })();
        if let Err(e) = result {
            log::warn!("could not remember the last login: {e}");
        }
    }
}

fn login_error(e: LoginError) -> ErrorBody {
    match e {
        LoginError::Auth => ErrorBody::new(ErrorCode::AuthFailed, "Incorrect username or password"),
        LoginError::Unsupported(prompt) => ErrorBody::new(
            ErrorCode::Failed,
            format!("This account needs another login step (“{}”), which isn't supported yet", prompt.trim()),
        ),
        LoginError::Failed(message) => {
            log::error!("login failed: {message}");
            ErrorBody::new(ErrorCode::Failed, "Login failed. See the system journal for details.")
        }
    }
}

fn session_env(session: &Session) -> Vec<String> {
    let mut env = vec!["XDG_SESSION_TYPE=wayland".to_owned()];
    let stem = session.info.id.trim_end_matches(".desktop");
    env.push(format!("XDG_SESSION_DESKTOP={stem}"));
    if let Some(names) = &session.desktop_names {
        env.push(format!("XDG_CURRENT_DESKTOP={}", names.trim_end_matches(';').replace(';', ":")));
    }
    env
}

/// Wayland sessions from the standard directories (or the colon-separated
/// `MERIDIAN_SESSION_DIRS`, for testing); earlier directories win.
fn load_sessions() -> Vec<Session> {
    let dirs: Vec<String> = match std::env::var("MERIDIAN_SESSION_DIRS") {
        Ok(v) => v.split(':').filter(|d| !d.is_empty()).map(str::to_owned).collect(),
        Err(_) => SESSION_DIRS.iter().map(|d| d.to_string()).collect(),
    };
    let mut sessions: Vec<Session> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut paths: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if let Some(session) = parse_session(&path)
                && !sessions.iter().any(|s| s.info.id == session.info.id)
            {
                sessions.push(session);
            }
        }
    }
    sessions.sort_by_cached_key(|s| (s.info.id != DEFAULT_SESSION, s.info.name.to_lowercase()));
    sessions
}

fn parse_session(path: &Path) -> Option<Session> {
    const GROUP: &str = "Desktop Entry";
    if path.extension()? != "desktop" {
        return None;
    }
    let kf = glib::KeyFile::new();
    kf.load_from_file(path, glib::KeyFileFlags::NONE).ok()?;
    if kf.boolean(GROUP, "Hidden").unwrap_or(false) || kf.boolean(GROUP, "NoDisplay").unwrap_or(false) {
        return None;
    }
    let exec = kf.string(GROUP, "Exec").ok()?;
    let cmd: Vec<String> =
        glib::shell_parse_argv(exec.as_str()).ok()?.into_iter().map(|a| a.to_string_lossy().into_owned()).collect();
    if cmd.is_empty() {
        return None;
    }
    Some(Session {
        info: SessionInfo {
            id: path.file_name()?.to_str()?.to_owned(),
            name: kf.locale_string(GROUP, "Name", None).ok()?.to_string(),
        },
        cmd,
        desktop_names: kf.string(GROUP, "DesktopNames").ok().map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_files() {
        let dir = std::env::temp_dir().join(format!("meridian-greeter-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plasma.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nName=Plasma (Wayland)\nExec=/usr/libexec/plasma-dbus-run-session-if-needed /usr/bin/startplasma-wayland\nDesktopNames=KDE\n",
        )
        .unwrap();
        let s = parse_session(&path).unwrap();
        assert_eq!(s.info.id, "plasma.desktop");
        assert_eq!(s.info.name, "Plasma (Wayland)");
        assert_eq!(s.cmd, vec!["/usr/libexec/plasma-dbus-run-session-if-needed", "/usr/bin/startplasma-wayland"]);
        let env = session_env(&s);
        assert!(env.contains(&"XDG_CURRENT_DESKTOP=KDE".to_owned()));
        assert!(env.contains(&"XDG_SESSION_DESKTOP=plasma".to_owned()));

        let hidden = dir.join("hidden.desktop");
        std::fs::write(&hidden, "[Desktop Entry]\nName=X\nExec=x\nHidden=true\n").unwrap();
        assert!(parse_session(&hidden).is_none());
    }

    #[test]
    fn multi_desktop_names_become_colon_separated() {
        let s = Session {
            info: SessionInfo { id: "meridian.desktop".into(), name: "Meridian".into() },
            cmd: vec!["meridian-session".into()],
            desktop_names: Some("Meridian;wlroots;".into()),
        };
        assert!(session_env(&s).contains(&"XDG_CURRENT_DESKTOP=Meridian:wlroots".to_owned()));
    }
}
