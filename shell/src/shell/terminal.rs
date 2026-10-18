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
