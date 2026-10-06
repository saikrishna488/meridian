//! Default output volume through WirePlumber's `wpctl`.
//!
//! M0 bridge: runs `wpctl` with a fixed argument vector (never a shell). M2
//! replaces this with the native PipeWire API (`pipewire-rs`) in
//! `meridian-servicesd`, which also gives change notifications.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SINK: &str = "@DEFAULT_AUDIO_SINK@";

type ErrorHandler = Rc<dyn Fn(String)>;

#[derive(Default)]
struct Inner {
    /// A `set-volume` is running; further requests coalesce into `pending`.
    in_flight: Cell<bool>,
    pending: Cell<Option<u8>>,
    on_error: RefCell<Option<ErrorHandler>>,
}

#[derive(Clone, Default)]
pub struct Audio {
    inner: Rc<Inner>,
}

impl Audio {
    pub fn new(on_error: impl Fn(String) + 'static) -> Self {
        let audio = Audio::default();
        *audio.inner.on_error.borrow_mut() = Some(Rc::new(on_error));
        audio
    }

    /// Read the current volume; `None` if wpctl or the sink is unavailable.
    pub fn read(&self, done: impl FnOnce(Option<u8>) + 'static) {
        let proc = match spawn(&["wpctl", "get-volume", SINK]) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("wpctl unavailable: {e}");
                return done(None);
            }
        };
        proc.communicate_utf8_async(None, gio::Cancellable::NONE, move |result| {
            let volume = match result {
                Ok((Some(stdout), _)) => parse_volume(&stdout),
                Ok(_) => None,
                Err(e) => {
                    log::warn!("wpctl get-volume: {e}");
                    None
                }
            };
            done(volume);
        });
    }

    /// Set the volume. Rapid calls (slider drags) coalesce: at most one
    /// `wpctl` runs at a time and the latest value wins.
    pub fn set(&self, percent: u8) {
        self.inner.pending.set(Some(percent.min(100)));
        if !self.inner.in_flight.get() {
            self.run_pending();
        }
    }

    fn run_pending(&self) {
        let Some(percent) = self.inner.pending.take() else { return };
        let level = format!("{}%", percent);
        let proc = match spawn(&["wpctl", "set-volume", SINK, &level]) {
            Ok(p) => p,
            Err(e) => return self.report(format!("wpctl unavailable: {e}")),
        };
        self.inner.in_flight.set(true);
        let this = self.clone();
        proc.wait_check_async(gio::Cancellable::NONE, move |result| {
            this.inner.in_flight.set(false);
            match result {
                // Raising the volume should be audible: unmute too.
                Ok(()) if percent > 0 => {
                    if let Err(e) = spawn(&["wpctl", "set-mute", SINK, "0"]) {
                        log::warn!("wpctl set-mute: {e}");
                    }
                }
                Ok(()) => {}
                Err(e) => this.report(format!("wpctl set-volume failed: {e}")),
            }
            this.run_pending();
        });
    }

    fn report(&self, message: String) {
        log::warn!("{message}");
        if let Some(f) = self.inner.on_error.borrow().clone() {
            f(message);
        }
    }
}

fn spawn(argv: &[&str]) -> Result<gio::Subprocess, glib::Error> {
    let argv: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    gio::Subprocess::newv(&argv, gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE)
}

/// Parse `wpctl get-volume` output: `Volume: 0.40` or `Volume: 0.40 [MUTED]`.
/// A muted sink reads as 0%.
fn parse_volume(output: &str) -> Option<u8> {
    let rest = output.trim().strip_prefix("Volume:")?.trim();
    let mut parts = rest.split_whitespace();
    let level: f64 = parts.next()?.parse().ok()?;
    if parts.any(|p| p == "[MUTED]") {
        return Some(0);
    }
    Some((level * 100.0).round().clamp(0.0, 100.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wpctl_output() {
        assert_eq!(parse_volume("Volume: 1.00\n"), Some(100));
        assert_eq!(parse_volume("Volume: 0.40"), Some(40));
        assert_eq!(parse_volume("Volume: 0.40 [MUTED]"), Some(0));
        assert_eq!(parse_volume("Volume: 1.50"), Some(100));
        assert_eq!(parse_volume("garbage"), None);
    }
}
