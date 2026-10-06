//! Typed messages exchanged between Meridian's web UI surfaces and the native
//! shell host.
//!
//! These types are the single source of truth for the UI/host boundary. The
//! TypeScript definitions in `ui/src/lib/generated/` are generated from them by
//! `ts-rs` whenever `cargo test -p meridian-protocol` runs.
//!
//! Wire format:
//! - request:  `{"method": "<name>", "params": {...}}` (params omitted for unit requests)
//! - reply:    `{"ok": true, "result": ...}` or `{"ok": false, "error": {...}}`
//! - event:    `{"event": "<name>", "data": ...}`

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The kinds of UI surfaces the shell hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum SurfaceKind {
    /// Wallpaper on every output.
    Wallpaper,
    /// Top bar: Applications button, running windows, Options button.
    Panel,
    /// Applications menu: search + list of installed applications.
    Applications,
    /// Options menu: Wi-Fi, Bluetooth, brightness, volume.
    Options,
    /// Login screen (`meridian-greeter`, a separate program run by greetd).
    Greeter,
}

/// Permissions granted to a surface. Every request requires at most one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum Capability {
    /// Query search providers and activate their results (launches apps).
    Search,
    /// Open/close the shell's menus (popovers).
    Popovers,
    /// List and activate running windows.
    Windows,
    /// Read and change system settings (Wi-Fi, Bluetooth, brightness, volume).
    Settings,
    /// Log in, list sessions, power off/restart from the login screen.
    Login,
}

impl SurfaceKind {
    /// The fixed capability set of each surface kind.
    pub fn capabilities(self) -> &'static [Capability] {
        match self {
            SurfaceKind::Wallpaper => &[],
            SurfaceKind::Panel => &[Capability::Popovers, Capability::Windows],
            SurfaceKind::Applications => &[Capability::Search, Capability::Popovers],
            SurfaceKind::Options => &[Capability::Popovers, Capability::Settings],
            SurfaceKind::Greeter => &[Capability::Login],
        }
    }

    pub fn has(self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }
}

/// The shell's menus, opened from the top bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum Popover {
    Applications,
    Options,
}

/// Identifies a search provider. Closed set: providers are native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum ProviderId {
    Apps,
}

/// A request sent by a UI surface to the host.
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(tag = "method", content = "params", deny_unknown_fields)]
#[ts(export)]
pub enum Request {
    /// Handshake. Marks the surface ready to receive events.
    #[serde(rename = "shell.hello")]
    Hello,

    /// Run a search. An empty query lists everything the providers offer
    /// by default (for apps: all applications, alphabetically).
    #[serde(rename = "search.query")]
    SearchQuery(SearchQuery),

    /// Activate a result previously returned by `search.query`.
    #[serde(rename = "search.activate")]
    SearchActivate(SearchActivate),

    /// Open the given menu, or close it if it is open.
    #[serde(rename = "popover.toggle")]
    PopoverToggle(PopoverToggle),

    /// Hide the requesting menu (sent after its exit animation).
    #[serde(rename = "popover.close")]
    PopoverClose,

    /// Currently open windows.
    #[serde(rename = "windows.list")]
    WindowsList,

    /// Focus (raise and unminimize) a window.
    #[serde(rename = "windows.activate")]
    WindowsActivate(WindowsActivate),

    /// Current settings state.
    #[serde(rename = "settings.get")]
    SettingsGet,

    #[serde(rename = "settings.set_wifi")]
    SettingsSetWifi(SetEnabled),

    #[serde(rename = "settings.set_bluetooth")]
    SettingsSetBluetooth(SetEnabled),

    #[serde(rename = "settings.set_brightness")]
    SettingsSetBrightness(SetPercent),

    #[serde(rename = "settings.set_volume")]
    SettingsSetVolume(SetPercent),

    /// Login screen: sessions, last user, hostname.
    #[serde(rename = "greeter.info")]
    GreeterInfo,

    /// Login screen: authenticate and start the chosen session. Replies
    /// once authentication finished (success, or the reason it failed).
    #[serde(rename = "greeter.login")]
    GreeterLogin(GreeterLogin),

    /// Login screen: power off or restart the machine.
    #[serde(rename = "greeter.power")]
    GreeterPower(GreeterPower),
}

#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct GreeterLogin {
    pub username: String,
    pub password: String,
    /// `SessionInfo::id`.
    pub session: String,
}

// Never print the password (requests are logged at trace level).
impl std::fmt::Debug for GreeterLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GreeterLogin")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("session", &self.session)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum PowerAction {
    PowerOff,
    Reboot,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct GreeterPower {
    pub action: PowerAction,
}

/// Reply to `greeter.info`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct GreeterInfo {
    pub hostname: String,
    pub sessions: Vec<SessionInfo>,
    pub last_user: Option<String>,
    /// `SessionInfo::id` of the last session started from this greeter.
    pub last_session: Option<String>,
}

/// A desktop session that can be started (from `wayland-sessions`).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SessionInfo {
    /// Session file name, e.g. `meridian.desktop`.
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SearchQuery {
    pub query: String,
    /// Echoed back in the results so the UI can discard stale responses.
    pub serial: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SearchActivate {
    pub provider: ProviderId,
    /// Opaque id from a `SearchItem`; resolved by the provider itself.
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PopoverToggle {
    pub popover: Popover,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WindowsActivate {
    /// `WindowInfo::id`.
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SetEnabled {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SetPercent {
    /// 0–100; larger values are rejected.
    pub percent: u8,
}

impl Request {
    /// The capability a surface must hold to make this request.
    pub fn required_capability(&self) -> Option<Capability> {
        match self {
            Request::Hello => None,
            Request::SearchQuery(_) | Request::SearchActivate(_) => Some(Capability::Search),
            Request::PopoverToggle(_) | Request::PopoverClose => Some(Capability::Popovers),
            Request::WindowsList | Request::WindowsActivate(_) => Some(Capability::Windows),
            Request::SettingsGet
            | Request::SettingsSetWifi(_)
            | Request::SettingsSetBluetooth(_)
            | Request::SettingsSetBrightness(_)
            | Request::SettingsSetVolume(_) => Some(Capability::Settings),
            Request::GreeterInfo | Request::GreeterLogin(_) | Request::GreeterPower(_) => Some(Capability::Login),
        }
    }
}

/// Reply to `shell.hello`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SurfaceInfo {
    pub surface: SurfaceKind,
    pub capabilities: Vec<Capability>,
    /// True when the shell runs with `MERIDIAN_DEV=1`.
    pub dev: bool,
}

/// Reply to `search.query`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SearchResults {
    pub serial: u32,
    pub sections: Vec<SearchSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SearchSection {
    pub provider: ProviderId,
    pub title: String,
    pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SearchItem {
    /// Opaque, provider-scoped id; pass back unchanged to `search.activate`.
    pub id: String,
    pub title: String,
    /// One line of secondary text (e.g. "Web Browser").
    pub subtitle: Option<String>,
}

/// An open window, as reported by the compositor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WindowInfo {
    /// Shell-assigned id, stable for the window's lifetime.
    pub id: u32,
    /// Application name (from its desktop entry when known, else its app id).
    pub name: String,
    pub title: String,
    /// Whether the window has focus.
    pub active: bool,
}

/// State of the Options menu. `null` means the control is unavailable
/// (no hardware, or the system service is not running).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, TS)]
#[ts(export)]
pub struct SettingsState {
    pub wifi: Option<bool>,
    pub bluetooth: Option<bool>,
    /// Display brightness, 0–100.
    pub brightness: Option<u8>,
    /// Default output volume, 0–100.
    pub volume: Option<u8>,
}

/// Which menus are open (sent to the panel to highlight its buttons).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PopoverState {
    pub applications: bool,
    pub options: bool,
}

/// Events pushed from the host to a surface.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "event", content = "data")]
#[ts(export)]
pub enum Event {
    /// To a menu: it was just shown.
    #[serde(rename = "popover.shown")]
    PopoverShown,
    /// To a menu: animate out, then send `popover.close`.
    #[serde(rename = "popover.dismiss")]
    PopoverDismiss,
    /// To the panel: which menus are open.
    #[serde(rename = "popover.state")]
    PopoverState(PopoverState),
    /// To the panel: the window list changed.
    #[serde(rename = "windows.changed")]
    WindowsChanged(Vec<WindowInfo>),
    /// Search results may have changed (apps installed/removed).
    #[serde(rename = "search.invalidated")]
    SearchInvalidated,
    /// To the Options menu: settings changed.
    #[serde(rename = "settings.changed")]
    SettingsChanged(SettingsState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorCode {
    /// Malformed or unknown request.
    InvalidRequest,
    /// The surface lacks the capability for this request.
    PermissionDenied,
    /// The referenced item (app id, window, …) does not exist.
    NotFound,
    /// The backing system service is unavailable.
    Unavailable,
    /// The operation failed.
    Failed,
    /// Wrong username or password.
    AuthFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

impl ErrorBody {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        ErrorBody { code, message: message.into() }
    }
}

#[derive(Serialize)]
struct OkReply<'a, T> {
    ok: bool,
    result: &'a T,
}

#[derive(Serialize)]
struct ErrReply<'a> {
    ok: bool,
    error: &'a ErrorBody,
}

/// Serialize a successful reply.
pub fn reply_ok<T: Serialize>(result: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(&OkReply { ok: true, result })
}

/// Serialize an error reply.
pub fn reply_err(error: &ErrorBody) -> String {
    serde_json::to_string(&ErrReply { ok: false, error }).expect("ErrorBody always serializes")
}

/// Parse a request from its JSON wire form, including range validation.
pub fn parse_request(json: &str) -> Result<Request, ErrorBody> {
    let request: Request =
        serde_json::from_str(json).map_err(|e| ErrorBody::new(ErrorCode::InvalidRequest, e.to_string()))?;
    match &request {
        Request::SettingsSetBrightness(p) | Request::SettingsSetVolume(p) if p.percent > 100 => {
            Err(ErrorBody::new(ErrorCode::InvalidRequest, "percent must be 0–100"))
        }
        _ => Ok(request),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unit_request_without_params() {
        assert_eq!(parse_request(r#"{"method":"shell.hello"}"#).unwrap(), Request::Hello);
    }

    #[test]
    fn parses_request_with_params() {
        let r = parse_request(r#"{"method":"search.activate","params":{"provider":"apps","item_id":"foot.desktop"}}"#)
            .unwrap();
        assert_eq!(
            r,
            Request::SearchActivate(SearchActivate { provider: ProviderId::Apps, item_id: "foot.desktop".into() })
        );
        assert_eq!(
            parse_request(r#"{"method":"popover.toggle","params":{"popover":"options"}}"#).unwrap(),
            Request::PopoverToggle(PopoverToggle { popover: Popover::Options })
        );
    }

    #[test]
    fn rejects_unknown_method() {
        let e = parse_request(r#"{"method":"shell.exec","params":{"cmd":"rm -rf ~"}}"#).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidRequest);
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(parse_request(r#"{"method":"search.query","params":{"query":"a","serial":1,"x":1}}"#).is_err());
        assert!(parse_request(r#"{"method":"shell.hello","extra":1}"#).is_err());
    }

    #[test]
    fn rejects_unknown_provider() {
        assert!(parse_request(r#"{"method":"search.activate","params":{"provider":"shell","item_id":"x"}}"#).is_err());
    }

    #[test]
    fn rejects_out_of_range_percent() {
        assert!(parse_request(r#"{"method":"settings.set_volume","params":{"percent":101}}"#).is_err());
        assert!(parse_request(r#"{"method":"settings.set_volume","params":{"percent":-1}}"#).is_err());
        assert!(parse_request(r#"{"method":"settings.set_brightness","params":{"percent":100}}"#).is_ok());
    }

    #[test]
    fn capability_gates() {
        let launch = Request::SearchActivate(SearchActivate { provider: ProviderId::Apps, item_id: "x".into() });
        let cap = launch.required_capability().unwrap();
        assert!(SurfaceKind::Applications.has(cap));
        assert!(!SurfaceKind::Panel.has(cap));
        assert!(!SurfaceKind::Options.has(cap));

        let wifi = Request::SettingsSetWifi(SetEnabled { enabled: false });
        let cap = wifi.required_capability().unwrap();
        assert!(SurfaceKind::Options.has(cap));
        assert!(!SurfaceKind::Applications.has(cap));
        assert!(!SurfaceKind::Panel.has(cap));

        assert!(SurfaceKind::Wallpaper.capabilities().is_empty());
    }

    #[test]
    fn login_is_greeter_only_and_password_is_redacted() {
        let r = parse_request(
            r#"{"method":"greeter.login","params":{"username":"sai","password":"hunter2","session":"meridian.desktop"}}"#,
        )
        .unwrap();
        let cap = r.required_capability().unwrap();
        assert!(SurfaceKind::Greeter.has(cap));
        for kind in [SurfaceKind::Panel, SurfaceKind::Applications, SurfaceKind::Options, SurfaceKind::Wallpaper] {
            assert!(!kind.has(cap), "{kind:?} must not be able to log in");
        }
        assert!(!format!("{r:?}").contains("hunter2"));
        // The greeter can do nothing else.
        assert_eq!(SurfaceKind::Greeter.capabilities(), &[Capability::Login]);
    }

    #[test]
    fn event_wire_format() {
        let json = serde_json::to_string(&Event::SettingsChanged(SettingsState::default())).unwrap();
        assert_eq!(
            json,
            r#"{"event":"settings.changed","data":{"wifi":null,"bluetooth":null,"brightness":null,"volume":null}}"#
        );
        assert_eq!(serde_json::to_string(&Event::PopoverShown).unwrap(), r#"{"event":"popover.shown"}"#);
    }

    #[test]
    fn reply_wire_format() {
        assert_eq!(reply_ok(&1).unwrap(), r#"{"ok":true,"result":1}"#);
        let e = reply_err(&ErrorBody::new(ErrorCode::NotFound, "no such app"));
        assert_eq!(e, r#"{"ok":false,"error":{"code":"not_found","message":"no such app"}}"#);
    }
}
