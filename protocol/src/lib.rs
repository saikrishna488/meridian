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

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct TerminalId {
    pub id: u32,
}

#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct TerminalWrite {
    pub id: u32,
    pub data: String,
}

#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct ClipboardText {
    pub data: String,
}
impl std::fmt::Debug for ClipboardText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClipboardText(<redacted>)")
    }
}

impl std::fmt::Debug for TerminalWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalWrite").field("id", &self.id).field("data", &"<redacted>").finish()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct TerminalSize {
    pub id: u32,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TerminalSession {
    pub id: u32,
    pub shell: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TerminalChunk {
    pub data: Vec<u8>,
    pub exited: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct FilePath {
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct FileList {
    pub path: String,
    pub hidden: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct FileName {
    pub path: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FileLocation {
    pub name: String,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub directory: bool,
    pub symlink: bool,
    pub size: f64,
    pub modified: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FileListing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<FileEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DesktopItem {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub target: String,
    pub icon: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct WallpaperChoice {
    pub id: String,
    pub name: String,
    pub uri: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DesktopState {
    pub appearance: String,
    pub wallpaper: String,
    pub wallpaper_uri: String,
    pub wallpapers: Vec<WallpaperChoice>,
    pub items: Vec<DesktopItem>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WallpaperSet {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DisplayMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DisplayOutput {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub current: Option<DisplayMode>,
    pub modes: Vec<DisplayMode>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DisplayConfig {
    pub name: String,
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DisplayApply {
    pub outputs: Vec<DisplayConfig>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DisplayTrial {
    pub token: String,
    pub seconds: u32,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DisplayDecision {
    pub token: String,
    pub keep: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct MonitorBrightnessRequest {
    pub connector: String,
    pub percent: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MonitorBrightness {
    pub percent: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub device: String,
    pub signal: u8,
    pub security: String,
    pub connected: bool,
}
#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WifiConnect {
    pub bssid: String,
    pub device: String,
    pub password: String,
}
impl std::fmt::Debug for WifiConnect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WifiConnect")
            .field("bssid", &self.bssid)
            .field("device", &self.device)
            .field("password", &"<redacted>")
            .finish()
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct BluetoothDevice {
    pub path: String,
    pub name: String,
    pub address: String,
    pub paired: bool,
    pub connected: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct BluetoothAction {
    pub path: String,
    pub action: String,
}
/// The kinds of UI surfaces the shell hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum SurfaceKind {
    /// Wallpaper on every output.
    Wallpaper,
    /// Top bar: Applications button, running windows, Options button.
    Panel,
    /// Desktop menu bar, anchored to the top edge.
    Topbar,
    /// Desktop dropdown menus and About dialog.
    DesktopMenu,
    /// Applications menu: search + list of installed applications.
    Applications,
    /// Control Center: connectivity, appearance, brightness, and volume.
    Options,
    /// Settings window for appearance, displays, and system devices.
    Preferences,
    /// File browser and installed-application manager.
    Finder,
    /// Interactive terminal, with a dedicated PTY capability.
    Terminal,
    /// Login screen (`meridian-greeter`, a separate program run by greetd).
    Greeter,
    /// Lock screen, shown on every output via the session-lock protocol
    /// while the session is locked.
    Locker,
}

/// Permissions granted to a surface. Every request requires at most one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum Capability {
    /// Query search providers and activate their results (launches apps).
    Search,
    /// Read and modify pinned applications.
    Dock,
    /// Preview and execute app uninstallation.
    AppManagement,
    /// Open/close the shell's menus (popovers).
    Popovers,
    /// List and activate running windows.
    Windows,
    /// Read and change system settings (Wi-Fi, Bluetooth, brightness, volume).
    Settings,
    /// Log in, list sessions, power off/restart from the login screen.
    Login,
    /// Lock, sign out, and suspend the running session.
    Session,
    /// Verify the password that ends a lock.
    Unlock,
    /// Browse and manage files from Finder.
    Files,
    /// Select the desktop wallpaper from Settings.
    Appearance,
    /// Manage this terminal window's interactive shells.
    Terminal,
}

impl SurfaceKind {
    /// The fixed capability set of each surface kind.
    pub fn capabilities(self) -> &'static [Capability] {
        match self {
            SurfaceKind::Wallpaper => &[Capability::Dock, Capability::Popovers],
            SurfaceKind::Preferences => &[Capability::Appearance, Capability::Settings],
            SurfaceKind::Finder => &[
                Capability::Files,
                Capability::Search,
                Capability::AppManagement,
                Capability::Popovers,
                Capability::Dock,
            ],
            SurfaceKind::Terminal => &[Capability::Terminal],
            SurfaceKind::Panel => &[
                Capability::Popovers,
                Capability::Windows,
                Capability::Search,
                Capability::Dock,
                Capability::AppManagement,
            ],
            SurfaceKind::Topbar => &[Capability::Popovers, Capability::Windows],
            SurfaceKind::DesktopMenu => &[Capability::Popovers, Capability::Windows, Capability::Session],
            SurfaceKind::Applications => &[
                Capability::Search,
                Capability::Popovers,
                Capability::Windows,
                Capability::Dock,
                Capability::AppManagement,
            ],
            SurfaceKind::Options => &[Capability::Popovers, Capability::Settings, Capability::Appearance],
            SurfaceKind::Greeter => &[Capability::Login],
            SurfaceKind::Locker => &[Capability::Unlock],
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
    DesktopMenu,
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
    #[serde(rename = "desktop.get")]
    DesktopGet,
    #[serde(rename = "desktop.appearance_set")]
    DesktopAppearanceSet(WallpaperSet),
    #[serde(rename = "desktop.wallpaper_set")]
    DesktopWallpaperSet(WallpaperSet),
    #[serde(rename = "desktop.wallpaper_choose")]
    DesktopWallpaperChoose,
    #[serde(rename = "desktop.add_app")]
    DesktopAddApp(AppId),
    #[serde(rename = "desktop.add_file")]
    DesktopAddFile(FilePath),
    #[serde(rename = "desktop.choose_files")]
    DesktopChooseFiles,
    #[serde(rename = "desktop.remove")]
    DesktopRemove(AppId),
    #[serde(rename = "desktop.open")]
    DesktopOpen(AppId),

    #[serde(rename = "preferences.open")]
    PreferencesOpen,
    #[serde(rename = "preferences.wallpaper_open")]
    PreferencesWallpaperOpen,
    #[serde(rename = "finder.open")]
    FinderOpen,
    #[serde(rename = "files.locations")]
    FileLocations,
    #[serde(rename = "files.list")]
    FileList(FileList),
    #[serde(rename = "files.open")]
    FileOpen(FilePath),
    #[serde(rename = "files.mkdir")]
    FileMkdir(FileName),
    #[serde(rename = "files.rename")]
    FileRename(FileName),
    #[serde(rename = "files.trash")]
    FileTrash(FilePath),
    #[serde(rename = "terminal.open")]
    TerminalOpen,
    #[serde(rename = "terminal.open_at")]
    TerminalOpenAt(FilePath),
    #[serde(rename = "terminal.start")]
    TerminalStart,
    #[serde(rename = "terminal.start_at")]
    TerminalStartAt(FilePath),
    #[serde(rename = "terminal.copy")]
    TerminalCopy(ClipboardText),
    #[serde(rename = "terminal.paste")]
    TerminalPaste,
    #[serde(rename = "terminal.read")]
    TerminalRead(TerminalId),
    #[serde(rename = "terminal.write")]
    TerminalWrite(TerminalWrite),
    #[serde(rename = "terminal.resize")]
    TerminalResize(TerminalSize),
    #[serde(rename = "terminal.close")]
    TerminalClose(TerminalId),
    #[serde(rename = "trash.open")]
    TrashOpen,
    #[serde(rename = "apps.uninstall_prompt")]
    AppUninstallPrompt(AppId),
    #[serde(rename = "apps.uninstall_plan")]
    AppUninstallPlan(AppId),
    #[serde(rename = "apps.uninstall")]
    AppUninstall(AppUninstall),

    #[serde(rename = "menu.open")]
    MenuOpen(MenuOpen),
    #[serde(rename = "system.info")]
    SystemInfo,

    #[serde(rename = "dock.layout_get")]
    DockLayoutGet,
    #[serde(rename = "dock.layout_set")]
    DockLayoutSet(DockLayout),
    #[serde(rename = "dock.list")]
    DockList,
    #[serde(rename = "dock.pin")]
    DockPin(DockPin),

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

    #[serde(rename = "settings.displays")]
    SettingsDisplays,
    #[serde(rename = "settings.display_apply")]
    SettingsDisplayApply(DisplayApply),
    #[serde(rename = "settings.display_decide")]
    SettingsDisplayDecide(DisplayDecision),
    #[serde(rename = "settings.monitor_brightness")]
    SettingsMonitorBrightness(MonitorBrightnessRequest),
    #[serde(rename = "settings.wifi_networks")]
    SettingsWifiNetworks,
    #[serde(rename = "settings.wifi_connect")]
    SettingsWifiConnect(WifiConnect),
    #[serde(rename = "settings.bluetooth_devices")]
    SettingsBluetoothDevices,
    #[serde(rename = "settings.bluetooth_scan")]
    SettingsBluetoothScan,
    #[serde(rename = "settings.bluetooth_action")]
    SettingsBluetoothAction(BluetoothAction),
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

    /// Lock the session: shows the lock screen on every output.
    #[serde(rename = "session.lock")]
    SessionLock,

    /// End the running session, back to the login screen.
    #[serde(rename = "session.sign_out")]
    SessionSignOut,

    /// Suspend the machine.
    #[serde(rename = "session.sleep")]
    SessionSleep,

    #[serde(rename = "session.restart")]
    SessionRestart,
    #[serde(rename = "session.shutdown")]
    SessionShutdown,

    /// Lock screen: check a password and end the lock if it's correct.
    #[serde(rename = "locker.unlock")]
    LockerUnlock(LockerUnlock),
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

#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LockerUnlock {
    pub password: String,
}

// Never print the password (requests are logged at trace level).
impl std::fmt::Debug for LockerUnlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockerUnlock").field("password", &"<redacted>").finish()
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
pub struct DockPin {
    pub id: String,
    pub pinned: bool,
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
            Request::DesktopGet => None,
            Request::DesktopAppearanceSet(_) | Request::DesktopWallpaperSet(_) | Request::DesktopWallpaperChoose => {
                Some(Capability::Appearance)
            }
            Request::DesktopAddFile(_) => Some(Capability::Dock),
            Request::DesktopAddApp(_)
            | Request::DesktopChooseFiles
            | Request::DesktopRemove(_)
            | Request::DesktopOpen(_) => Some(Capability::Dock),
            Request::FileLocations
            | Request::FileList(_)
            | Request::FileOpen(_)
            | Request::FileMkdir(_)
            | Request::FileRename(_)
            | Request::FileTrash(_) => Some(Capability::Files),
            Request::TerminalStart
            | Request::TerminalStartAt(_)
            | Request::TerminalCopy(_)
            | Request::TerminalPaste
            | Request::TerminalRead(_)
            | Request::TerminalWrite(_)
            | Request::TerminalResize(_)
            | Request::TerminalClose(_) => Some(Capability::Terminal),
            Request::TerminalOpenAt(_) => Some(Capability::Files),
            Request::PreferencesOpen
            | Request::PreferencesWallpaperOpen
            | Request::FinderOpen
            | Request::TerminalOpen => Some(Capability::Popovers),
            Request::TrashOpen => Some(Capability::Dock),
            Request::AppUninstallPrompt(_) | Request::AppUninstallPlan(_) | Request::AppUninstall(_) => {
                Some(Capability::AppManagement)
            }
            Request::MenuOpen(_) | Request::SystemInfo => Some(Capability::Popovers),
            Request::DockLayoutGet | Request::DockLayoutSet(_) | Request::DockList | Request::DockPin(_) => {
                Some(Capability::Dock)
            }
            Request::SearchQuery(_) | Request::SearchActivate(_) => Some(Capability::Search),
            Request::PopoverToggle(_) | Request::PopoverClose => Some(Capability::Popovers),
            Request::WindowsList | Request::WindowsActivate(_) => Some(Capability::Windows),
            Request::SettingsDisplays
            | Request::SettingsDisplayApply(_)
            | Request::SettingsDisplayDecide(_)
            | Request::SettingsMonitorBrightness(_)
            | Request::SettingsWifiNetworks
            | Request::SettingsWifiConnect(_)
            | Request::SettingsBluetoothDevices
            | Request::SettingsBluetoothScan
            | Request::SettingsBluetoothAction(_)
            | Request::SettingsGet
            | Request::SettingsSetWifi(_)
            | Request::SettingsSetBluetooth(_)
            | Request::SettingsSetBrightness(_)
            | Request::SettingsSetVolume(_) => Some(Capability::Settings),
            Request::GreeterInfo | Request::GreeterLogin(_) | Request::GreeterPower(_) => Some(Capability::Login),
            Request::SessionLock
            | Request::SessionSignOut
            | Request::SessionSleep
            | Request::SessionRestart
            | Request::SessionShutdown => Some(Capability::Session),
            Request::LockerUnlock(_) => Some(Capability::Unlock),
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
    /// Serialized GIO icon, resolved by the host icon theme.
    pub icon: Option<String>,
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
    #[serde(rename = "terminal.opened")]
    TerminalOpened,
    #[serde(rename = "terminal.opened_at")]
    TerminalOpenedAt(FilePath),
    #[serde(rename = "terminal.closed")]
    TerminalClosed,
    #[serde(rename = "finder.location")]
    FinderLocation(FilePath),
    #[serde(rename = "preferences.wallpaper")]
    PreferencesWallpaper,
    #[serde(rename = "desktop.changed")]
    DesktopChanged(DesktopState),
    #[serde(rename = "dock.layout_changed")]
    DockLayoutChanged(Vec<String>),
    #[serde(rename = "apps.uninstall_requested")]
    AppUninstallRequested(AppId),
    #[serde(rename = "menu.opened")]
    MenuOpened(MenuOpen),
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
    #[serde(rename = "dock.changed")]
    DockChanged(Vec<String>),
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
        assert!(SurfaceKind::Panel.has(cap));
        assert!(!SurfaceKind::Options.has(cap));

        let wifi = Request::SettingsSetWifi(SetEnabled { enabled: false });
        let cap = wifi.required_capability().unwrap();
        assert!(SurfaceKind::Options.has(cap));
        assert!(!SurfaceKind::Applications.has(cap));
        assert!(!SurfaceKind::Panel.has(cap));

        assert!(!SurfaceKind::Wallpaper.has(Capability::Files));
        assert!(SurfaceKind::Finder.has(Capability::Files));
        assert!(!SurfaceKind::Panel.has(Capability::Files));
        assert!(!SurfaceKind::Preferences.has(Capability::Files));
        assert_eq!(SurfaceKind::Terminal.capabilities(), &[Capability::Terminal]);
        for kind in [
            SurfaceKind::Preferences,
            SurfaceKind::Finder,
            SurfaceKind::Applications,
            SurfaceKind::Panel,
            SurfaceKind::Greeter,
            SurfaceKind::Locker,
        ] {
            assert!(!kind.has(Capability::Terminal));
        }
    }

    #[test]
    fn login_is_greeter_only_and_password_is_redacted() {
        let r = parse_request(
            r#"{"method":"greeter.login","params":{"username":"sai","password":"hunter2","session":"meridian.desktop"}}"#,
        )
        .unwrap();
        let cap = r.required_capability().unwrap();
        assert!(SurfaceKind::Greeter.has(cap));
        for kind in [
            SurfaceKind::Panel,
            SurfaceKind::Applications,
            SurfaceKind::Options,
            SurfaceKind::Wallpaper,
            SurfaceKind::Locker,
        ] {
            assert!(!kind.has(cap), "{kind:?} must not be able to log in");
        }
        assert!(!format!("{r:?}").contains("hunter2"));
        // The greeter can do nothing else.
        assert_eq!(SurfaceKind::Greeter.capabilities(), &[Capability::Login]);
    }

    #[test]
    fn locker_unlock_is_locker_only_and_password_is_redacted() {
        let r = parse_request(r#"{"method":"locker.unlock","params":{"password":"hunter2"}}"#).unwrap();
        let cap = r.required_capability().unwrap();
        assert!(SurfaceKind::Locker.has(cap));
        for kind in [SurfaceKind::Panel, SurfaceKind::Applications, SurfaceKind::Options, SurfaceKind::Wallpaper] {
            assert!(!kind.has(cap), "{kind:?} must not be able to answer the lock screen");
        }
        assert!(!format!("{r:?}").contains("hunter2"));
        // The locker can do nothing else.
        assert_eq!(SurfaceKind::Locker.capabilities(), &[Capability::Unlock]);
    }

    #[test]
    fn session_actions_are_available_to_the_meridian_menu() {
        for r in [
            Request::SessionLock,
            Request::SessionSignOut,
            Request::SessionSleep,
            Request::SessionRestart,
            Request::SessionShutdown,
        ] {
            let cap = r.required_capability().unwrap();
            assert!(!SurfaceKind::Options.has(cap));
            assert!(SurfaceKind::DesktopMenu.has(cap));
            assert!(!SurfaceKind::Applications.has(cap));
            assert!(!SurfaceKind::Greeter.has(cap));
        }
    }

    #[test]
    fn settings_device_requests_are_gated_and_wifi_password_is_redacted() {
        let request = parse_request(r#"{"method":"settings.wifi_connect","params":{"bssid":"AA:BB:CC:DD:EE:FF","device":"wlan0","password":"secret-password"}}"#).unwrap();
        assert!(SurfaceKind::Preferences.has(request.required_capability().unwrap()));
        assert!(!SurfaceKind::Finder.has(request.required_capability().unwrap()));
        assert!(!format!("{request:?}").contains("secret-password"));
        for request in [Request::SettingsDisplays, Request::SettingsWifiNetworks, Request::SettingsBluetoothDevices] {
            assert_eq!(request.required_capability(), Some(Capability::Settings));
        }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct MenuOpen {
    pub menu: String,
    pub x: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SystemInfo {
    pub version: String,
    pub manufacturer: String,
    pub model: String,
    pub os: String,
    pub processor: String,
    pub memory: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct AppId {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct AppUninstall {
    pub id: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct UninstallPlan {
    pub id: String,
    pub name: String,
    pub target: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DockLayout {
    pub items: Vec<String>,
}
