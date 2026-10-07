//! Interactive shells on real PTYs. Reads are bounded and pulled by the UI
//! after rendering, so a verbose command cannot flood the WebKit event queue.
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use meridian_protocol::{ErrorBody, ErrorCode, TerminalChunk, TerminalSession, TerminalSize};

const MAX_SESSIONS: usize = 16;
const MAX_INPUT: usize = 1024 * 1024;

#[derive(Default)]
pub struct Terminals {
    sessions: HashMap<u32, Pty>,
    next_id: u32,
}

impl Terminals {
    pub fn start(&mut self) -> Result<TerminalSession, ErrorBody> {
        self.start_in(None)
    }
    pub fn start_at(&mut self, path: &str) -> Result<TerminalSession, ErrorBody> {
        let cwd = std::fs::canonicalize(path).map_err(failed)?;
        if !cwd.is_dir() {
            return Err(invalid("The selected folder is no longer available."));
        }
        self.start_in(Some(cwd))
    }
    fn start_in(&mut self, cwd: Option<std::path::PathBuf>) -> Result<TerminalSession, ErrorBody> {
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(invalid("Close a terminal tab before opening another."));
        }
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|p| p.starts_with('/') && std::path::Path::new(p).is_file())
            .unwrap_or_else(|| "/bin/bash".into());
        let pty = Pty::spawn(&shell, cwd.as_deref()).map_err(failed)?;
        self.next_id = self.next_id.checked_add(1).ok_or_else(|| invalid("Terminal session limit reached."))?;
        let id = self.next_id;
        self.sessions.insert(id, pty);
        Ok(TerminalSession { id, shell })
    }
    fn get(&mut self, id: u32) -> Result<&mut Pty, ErrorBody> {
        self.sessions.get_mut(&id).ok_or_else(|| invalid("This terminal session has closed."))
    }
    pub fn read(&mut self, id: u32) -> Result<TerminalChunk, ErrorBody> {
        self.get(id)?.read().map_err(failed)
    }
    pub fn write(&mut self, id: u32, data: &str) -> Result<(), ErrorBody> {
        let pty = self.get(id)?;
        if data.len() + pty.pending.len() > MAX_INPUT {
            return Err(invalid("Terminal input is too large."));
        }
        pty.pending.extend(data.as_bytes());
        pty.flush().map_err(failed)
    }
    pub fn resize(&mut self, size: TerminalSize) -> Result<(), ErrorBody> {
        if !(2..=500).contains(&size.cols) || !(1..=250).contains(&size.rows) {
            return Err(invalid("Invalid terminal size."));
        }
        self.get(size.id)?.resize(size.cols, size.rows).map_err(failed)
    }
    pub fn close(&mut self, id: u32) {
        self.sessions.remove(&id);
    }
    pub fn clear(&mut self) {
        self.sessions.clear();
    }
}

struct Pty {
    master: File,
    child: Option<Child>,
    pending: VecDeque<u8>,
    eof: bool,
}

impl Pty {
    fn spawn(shell: &str, cwd: Option<&std::path::Path>) -> std::io::Result<Self> {
        let mut master = -1;
        let mut slave = -1;
        let size = libc::winsize { ws_row: 24, ws_col: 80, ws_xpixel: 0, ws_ypixel: 0 };
        // SAFETY: openpty writes two fresh descriptors into valid pointers.
        if unsafe { libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null(), &size) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: these descriptors are newly owned by us.
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        for fd in [master.as_raw_fd(), slave.as_raw_fd()] {
            // SAFETY: fd is live; the command's stdio duplication clears CLOEXEC on 0/1/2.
            if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        let mut command = Command::new(shell);
        command
            .arg("-i")
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave));
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        } else if let Some(home) = std::env::var_os("HOME") {
            command.current_dir(home);
        }
        // SAFETY: the child only calls async-signal-safe syscalls between fork and exec.
        // Establish a separate session and controlling terminal for job control / Ctrl+C.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                for signal in [libc::SIGINT, libc::SIGQUIT, libc::SIGTERM, libc::SIGHUP, libc::SIGPIPE, libc::SIGCHLD] {
                    libc::signal(signal, libc::SIG_DFL);
                }
                let mut mask = std::mem::zeroed();
                libc::sigemptyset(&mut mask);
                libc::sigprocmask(libc::SIG_SETMASK, &mask, std::ptr::null_mut());
                Ok(())
            });
        }
        // Make only the master's file description nonblocking (the slave remains blocking).
        // SAFETY: master is a live owned fd.
        if unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let child = command.spawn()?;
        Ok(Self { master, child: Some(child), pending: VecDeque::new(), eof: false })
    }
    fn resize(&self, cols: u16, rows: u16) -> std::io::Result<()> {
        let size = libc::winsize { ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 };
        // SAFETY: a valid fd and pointer; the kernel also sends SIGWINCH.
        if unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &size) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        while !self.pending.is_empty() {
            let (data, _) = self.pending.as_slices();
            match self.master.write(data) {
                Ok(0) => break,
                Ok(n) => {
                    self.pending.drain(..n);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn read(&mut self) -> std::io::Result<TerminalChunk> {
        if !self.eof {
            self.flush()?;
        }
        let mut data = vec![0; 32768];
        let n = match self.master.read(&mut data) {
            Ok(n) => {
                if n == 0 {
                    self.eof = true;
                }
                n
            }
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted) => 0,
            Err(e) if e.raw_os_error() == Some(libc::EIO) => {
                self.eof = true;
                0
            }
            Err(e) => return Err(e),
        };
        data.truncate(n);
        if let Some(child) = &mut self.child {
            let _ = child.try_wait()?;
        }
        Ok(TerminalChunk { data, exited: self.eof && n == 0 })
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Closing a tab ends its foreground job and shell; detached jobs may survive.
            // SAFETY: the PTY reports its own foreground process group.
            unsafe {
                let foreground = libc::tcgetpgrp(self.master.as_raw_fd());
                if foreground > 0 {
                    libc::kill(-foreground, libc::SIGHUP);
                }
                if child.try_wait().ok().flatten().is_none() {
                    libc::kill(-(child.id() as i32), libc::SIGHUP);
                }
            }
            // Never wait on the UI thread. Reap even a shell that ignores SIGHUP.
            std::thread::spawn(move || {
                for _ in 0..20 {
                    if child.try_wait().ok().flatten().is_some() {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                let _ = child.kill();
                let _ = child.wait();
            });
        }
    }
}
fn invalid(message: &str) -> ErrorBody {
    ErrorBody::new(ErrorCode::InvalidRequest, message)
}
fn failed(error: std::io::Error) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, format!("Terminal: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn until(pty: &mut Pty, marker: &str) -> String {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut bytes = Vec::new();
        while std::time::Instant::now() < deadline {
            bytes.extend(pty.read().unwrap().data);
            let output = String::from_utf8_lossy(&bytes);
            if output.contains(marker) {
                return output.into_owned();
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("Expected {marker}: {}", String::from_utf8_lossy(&bytes));
    }
    #[test]
    fn interactive_shell_preserves_cwd_and_handles_interrupt_and_resize() {
        let mut pty = Pty::spawn("/bin/bash", None).unwrap();
        // Disable echo to distinguish command text from actual command output.
        pty.pending.extend(b"stty -echo; printf 'READY\\n'\n");
        pty.flush().unwrap();
        until(&mut pty, "READY\r\n");
        pty.pending.extend(b"cd /tmp; test -t 0 && printf 'TTY_OK\\n'; printf 'CWD=%s\\n' \"$PWD\"\n");
        pty.flush().unwrap();
        let output = until(&mut pty, "CWD=/tmp");
        assert!(output.contains("TTY_OK"));
        pty.resize(100, 30).unwrap();
        pty.pending.extend(b"stty size\n");
        pty.flush().unwrap();
        until(&mut pty, "30 100");
        pty.pending.extend(b"sleep 30\n");
        pty.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        pty.pending.extend(b"\x03printf 'INTERRUPT_OK\\n'\n");
        pty.flush().unwrap();
        until(&mut pty, "INTERRUPT_OK\r\n");
    }
    #[test]
    fn rejects_unknown_sessions_and_invalid_sizes() {
        let mut sessions = Terminals::default();
        assert!(sessions.write(42, "echo x").is_err());
        assert!(sessions.resize(TerminalSize { id: 42, cols: 0, rows: 0 }).is_err());
    }
}
