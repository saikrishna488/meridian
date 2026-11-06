//! Terminal feature handler. Authorization and lock gating run in the parent dispatcher.
use super::*;

impl Shell {
    pub(super) fn dispatch_terminal(&self, request: Request) -> Result<Reply, ErrorBody> {
        match request {
            Request::TerminalOpen => {
                self.open_terminal();
                serialize(ok())
            }
            Request::TerminalOpenAt(path) => {
                let path = crate::files::directory_path(&path.path)?;
                self.open_terminal_at(path.to_string_lossy().into_owned());
                serialize(ok())
            }
            Request::TerminalLaunch(config) => serialize(proto::reply_ok(&self.terminals.borrow_mut().launch(config)?)),
            Request::TerminalInfo(id) => serialize(proto::reply_ok(&self.terminals.borrow_mut().info(id.id)?)),
            Request::TerminalQuit => {
                let window = self.state.borrow().terminal.as_ref().map(|s| s.window.clone());
                if let Some(window) = window { window.close(); }
                serialize(ok())
            }
            Request::TerminalNotify(text) => {
                if text.data.len() > 512 { return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Notification too long")); }
                let notification = gio::Notification::new("Terminal command finished");
                notification.set_body(Some(&text.data));
                self.app.send_notification(Some("terminal-command"), &notification);
                serialize(ok())
            }
            Request::TerminalLink(link) => {
                if link.target.len() > 8192 || link.target.chars().any(char::is_control) { return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Invalid link")); }
                if link.target.starts_with("https://") || link.target.starts_with("http://") {
                    gio::AppInfo::launch_default_for_uri(&link.target, None::<&gio::AppLaunchContext>)
                        .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?;
                } else {
                    let cwd = self.terminals.borrow_mut().info(link.id)?.cwd;
                    let target = if let Some(rest) = link.target.strip_prefix("~/") { glib::home_dir().join(rest) }
                        else { std::path::Path::new(&cwd).join(&link.target) };
                    let target = std::fs::canonicalize(target).map_err(|e| ErrorBody::new(ErrorCode::NotFound, e.to_string()))?;
                    if link.folder || target.is_dir() {
                        let directory = if target.is_dir() { target.as_path() } else { target.parent().unwrap_or(&target) };
                        self.open_finder_at(&directory.to_string_lossy());
                    } else {
                        let editor = gio::AppInfo::default_for_type("text/plain", false)
                            .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "Choose a default text editor in Settings."))?;
                        editor.launch(&[gio::File::for_path(target)], None::<&gio::AppLaunchContext>)
                            .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?;
                    }
                }
                serialize(ok())
            }
            Request::TerminalStart => serialize(proto::reply_ok(&self.terminals.borrow_mut().start()?)),
            Request::TerminalStartAt(path) => {
                serialize(proto::reply_ok(&self.terminals.borrow_mut().start_at(&path.path)?))
            }
            Request::TerminalCopy(text) => {
                if text.data.len() > 1024 * 1024 {
                    return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Clipboard text is too large."));
                }
                gdk::Display::default()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "No clipboard"))?
                    .clipboard()
                    .set_text(&text.data);
                serialize(ok())
            }
            Request::TerminalPaste => {
                let clipboard = gdk::Display::default()
                    .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "No clipboard"))?
                    .clipboard();
                Ok(Reply::Later(Box::pin(async move {
                    let text = clipboard
                        .read_text_future()
                        .await
                        .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?
                        .map(|t| t.to_string());
                    if text.as_ref().is_some_and(|t| t.len() > 1024 * 1024) {
                        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Clipboard text is too large."));
                    }
                    proto::reply_ok(&text).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::TerminalRead(session) => {
                serialize(proto::reply_ok(&self.terminals.borrow_mut().read(session.id)?))
            }
            Request::TerminalWrite(input) => {
                self.terminals.borrow_mut().write(input.id, &input.data)?;
                serialize(ok())
            }
            Request::TerminalResize(size) => {
                self.terminals.borrow_mut().resize(size)?;
                serialize(ok())
            }
            Request::TerminalClose(session) => {
                self.terminals.borrow_mut().close(session.id);
                serialize(ok())
            }
            _ => Err(ErrorBody::new(ErrorCode::InvalidRequest, "Not a terminal request")),
        }
    }
}
