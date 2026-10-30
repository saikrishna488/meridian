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

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct FileProperties {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: Option<f64>,
    pub modified: Option<f64>,
    pub created: Option<f64>,
    pub owner: String,
    pub group: String,
    pub permissions: String,
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TrashEntry {
    pub uri: String,
    pub name: String,
    pub original: String,
    pub deleted: String,
    pub size: f64,
    pub directory: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TrashListing {
    pub entries: Vec<TrashEntry>,
    pub truncated: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct TrashId {
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct StartupEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,
    pub enabled: bool,
    pub available: bool,
    pub reason: String,
    pub user: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct StartupSet {
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub default: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AudioState {
    pub devices: Vec<AudioDevice>,
    pub output_volume: Option<u8>,
    pub input_volume: Option<u8>,
    pub output_muted: Option<bool>,
    pub input_muted: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct AudioAction {
    pub id: u32,
    pub name: String,
    pub action: String,
    pub percent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct BatteryInfo {
    pub name: String,
    pub percent: Option<u8>,
    pub status: String,
    pub health: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PowerState {
    pub on_ac: Option<bool>,
    pub batteries: Vec<BatteryInfo>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct IdleSettings {
    pub lock_after: u32,
    pub blank_after: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[ts(export)]
pub enum PreferencesPage {
    Home,
    Sound,
    Power,
    StartupApps,
    Wifi,
    Bluetooth,
    Displays,
    Appearance,
    Wallpaper,
    Desktop,
    Applications,
    General,
    Users,
    DeviceManager,
    Search,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PreferencesPageRequest {
    pub page: PreferencesPage,
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
    pub font: String,
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
    pub frequency: u32,
    pub channel: u32,
    pub rate: String,
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
pub struct WifiProfile {
    pub uuid: String,
    pub name: String,
    pub device: String,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WifiProfileQuery {
    pub uuid: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WifiConfig {
    pub uuid: String,
    pub autoconnect: bool,
    pub metered: String,
    pub method: String,
    pub addresses: String,
    pub gateway: String,
    pub dns: String,
    pub ignore_auto_dns: bool,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WifiProfileAction {
    pub uuid: String,
    pub action: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct BluetoothDevice {
    pub path: String,
    pub name: String,
    pub address: String,
    pub trusted: bool,
    pub icon: String,
    pub battery: Option<u8>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum BluetoothAudioMode {
    Auto,
    LowQuality,
    MediumQuality,
    #[serde(alias = "highest-quality")]
    HighQuality,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct BluetoothAudioState {
    pub mode: BluetoothAudioMode,
    pub active_codec: Option<String>,
    pub active_device: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct BluetoothAudioQuality {
    pub mode: BluetoothAudioMode,
}
/// The kinds of UI surfaces the shell hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum SurfaceKind {
    /// Wallpaper on every output.
    Wallpaper,
    /// Bottom dock: Applications button, pinned/running windows, and Trash.
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
    /// Pulse system monitor.
    Pulse,
    Calendar,
    Clock,
    /// Notification center.
    Notifications,
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
    /// Manage applications that start at login, only from Settings.
    Startup,
    /// Read laptop battery and external power status.
    PowerStatus,
    /// Privileged driver and local account management, only in Settings.
    SystemAdministration,
    /// Log in, list sessions, power off/restart from the login screen.
    Login,
    /// Lock, sign out, and suspend the running session.
    Session,
    /// Verify the password that ends a lock.
    Unlock,
    /// Browse and manage files from Finder.
    Files,
    /// Manage desktop files without granting full file-manager access.
    DesktopFiles,
    /// Select the desktop wallpaper from Settings.
    Appearance,
    /// Manage this terminal window's interactive shells.
    Terminal,
    /// Read-only system resource and process metrics.
    Monitor,
    Notifications,
    Calendar,
}

impl SurfaceKind {
    /// The fixed capability set of each surface kind.
    pub fn capabilities(self) -> &'static [Capability] {
        match self {
            SurfaceKind::Wallpaper => &[Capability::Dock, Capability::DesktopFiles, Capability::Popovers],
            SurfaceKind::Preferences => &[
                Capability::Appearance,
                Capability::Settings,
                Capability::Startup,
                Capability::PowerStatus,
                Capability::SystemAdministration,
                Capability::Search,
                Capability::AppManagement,
            ],
            SurfaceKind::Finder => &[
                Capability::Files,
                Capability::DesktopFiles,
                Capability::Search,
                Capability::AppManagement,
                Capability::Popovers,
                Capability::Dock,
            ],
            SurfaceKind::Terminal => &[Capability::Terminal],
            SurfaceKind::Notifications => &[Capability::Notifications, Capability::Popovers],
            SurfaceKind::Pulse => &[Capability::Monitor],
            SurfaceKind::Calendar => &[Capability::Calendar],
            SurfaceKind::Clock => &[],
            SurfaceKind::Panel => &[
                Capability::Popovers,
                Capability::Windows,
                Capability::Search,
                Capability::Dock,
                Capability::AppManagement,
            ],
            SurfaceKind::Topbar => &[
                Capability::Popovers,
                Capability::Windows,
                Capability::PowerStatus,
                Capability::Notifications,
                Capability::Search,
            ],
            SurfaceKind::DesktopMenu => {
                &[Capability::Popovers, Capability::Windows, Capability::Session, Capability::Search, Capability::Dock]
            }
            SurfaceKind::Applications => &[
                Capability::Search,
                Capability::Popovers,
                Capability::Windows,
                Capability::Dock,
                Capability::AppManagement,
            ],
            SurfaceKind::Options => &[
                Capability::Notifications,
                Capability::Popovers,
                Capability::Settings,
                Capability::PowerStatus,
                Capability::Appearance,
                Capability::Session,
            ],
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
    Notifications,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PrimaryDisplayChoice {
    pub connector: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrimaryDisplayState {
    pub connector: Option<String>,
    pub effective: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SearchPreferences {
    pub settings: bool,
    pub calculator: bool,
}
impl Default for SearchPreferences {
    fn default() -> Self {
        Self { settings: true, calculator: false }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FilePreview {
    pub kind: String,
    pub content: String,
    pub truncated: bool,
}

/// Identifies a search provider. Closed set: providers are native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum ProviderId {
    Apps,
    Settings,
    Calculator,
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
    #[serde(rename = "desktop.font_set")]
    DesktopFontSet(WallpaperSet),
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
    #[serde(rename = "preferences.page_open")]
    PreferencesPageOpen(PreferencesPageRequest),
    #[serde(rename = "finder.open")]
    FinderOpen,
    #[serde(rename = "pulse.open")]
    PulseOpen,
    #[serde(rename = "pulse.snapshot")]
    PulseSnapshot,
    #[serde(rename = "calendar.list")]
    CalendarList,
    #[serde(rename = "calendar.save")]
    CalendarSave(Vec<CalendarEvent>),
    #[serde(rename = "notifications.get")]
    NotificationsGet,
    #[serde(rename = "notifications.read")]
    NotificationsRead,
    #[serde(rename = "notifications.do_not_disturb")]
    NotificationsDoNotDisturb(SetEnabled),
    #[serde(rename = "notifications.dismiss")]
    NotificationsDismiss(NotificationId),
    #[serde(rename = "notifications.clear")]
    NotificationsClear,
    #[serde(rename = "notifications.action")]
    NotificationsAction(NotificationAction),
    #[serde(rename = "files.locations")]
    FileLocations,
    #[serde(rename = "files.bookmarks")]
    FileBookmarks,
    #[serde(rename = "files.bookmark_set")]
    FileBookmarkSet(FilePath),
    #[serde(rename = "files.choose_folder")]
    FileChooseFolder,
    #[serde(rename = "files.list")]
    FileList(FileList),
    #[serde(rename = "files.properties")]
    FileProperties(FilePath),
    #[serde(rename = "files.trash_list")]
    FileTrashList,
    #[serde(rename = "files.trash_restore")]
    FileTrashRestore(TrashId),
    #[serde(rename = "files.open")]
    FileOpen(FilePath),
    #[serde(rename = "files.mkdir")]
    FileMkdir(FileName),
    #[serde(rename = "files.rename")]
    FileRename(FileName),
    #[serde(rename = "files.transfer")]
    FileTransfer(FileTransfer),
    #[serde(rename = "files.transfer_start")]
    FileTransferStart(FileTransfer),
    #[serde(rename = "files.transfer_status")]
    FileTransferStatus(TransferId),
    #[serde(rename = "files.transfer_cancel")]
    FileTransferCancel(TransferId),
    #[serde(rename = "files.transfers")]
    FileTransfers,
    #[serde(rename = "files.thumbnail")]
    FileThumbnail(FilePath),
    #[serde(rename = "files.preview")]
    FilePreview(FilePath),
    #[serde(rename = "files.wallpaper")]
    FileWallpaper(FilePath),
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

    /// Request a graceful close, allowing the app to prompt for unsaved work.
    #[serde(rename = "windows.close")]
    WindowsClose(WindowsActivate),

    #[serde(rename = "settings.displays")]
    SettingsDisplays,
    #[serde(rename = "settings.primary_display")]
    SettingsPrimaryDisplay,
    #[serde(rename = "settings.primary_display_set")]
    SettingsPrimaryDisplaySet(PrimaryDisplayChoice),
    #[serde(rename = "settings.search")]
    SettingsSearch,
    #[serde(rename = "settings.search_set")]
    SettingsSearchSet(SearchPreferences),
    #[serde(rename = "settings.display_apply")]
    SettingsDisplayApply(DisplayApply),
    #[serde(rename = "settings.display_decide")]
    SettingsDisplayDecide(DisplayDecision),
    #[serde(rename = "settings.monitor_brightness")]
    SettingsMonitorBrightness(MonitorBrightnessRequest),
    #[serde(rename = "settings.wifi_networks")]
    SettingsWifiNetworks,
    #[serde(rename = "settings.wifi_scan")]
    SettingsWifiScan,
    #[serde(rename = "settings.wifi_profiles")]
    SettingsWifiProfiles,
    #[serde(rename = "settings.wifi_config")]
    SettingsWifiConfig(WifiProfileQuery),
    #[serde(rename = "settings.wifi_config_set")]
    SettingsWifiConfigSet(WifiConfig),
    #[serde(rename = "settings.wifi_profile_action")]
    SettingsWifiProfileAction(WifiProfileAction),
    #[serde(rename = "settings.wifi_connect")]
    SettingsWifiConnect(WifiConnect),
    #[serde(rename = "settings.bluetooth_devices")]
    SettingsBluetoothDevices,
    #[serde(rename = "settings.bluetooth_scan")]
    SettingsBluetoothScan,
    #[serde(rename = "settings.bluetooth_action")]
    SettingsBluetoothAction(BluetoothAction),
    #[serde(rename = "settings.bluetooth_audio")]
    SettingsBluetoothAudio,
    #[serde(rename = "settings.bluetooth_audio_set")]
    SettingsBluetoothAudioSet(BluetoothAudioQuality),
    /// Current settings state.
    #[serde(rename = "settings.get")]
    SettingsGet,
    #[serde(rename = "power.status")]
    PowerStatus,
    #[serde(rename = "settings.idle")]
    SettingsIdle,
    #[serde(rename = "settings.idle_set")]
    SettingsIdleSet(IdleSettings),
    #[serde(rename = "settings.audio")]
    SettingsAudio,
    #[serde(rename = "settings.audio_action")]
    SettingsAudioAction(AudioAction),
    #[serde(rename = "settings.startup")]
    SettingsStartup,
    #[serde(rename = "settings.startup_set")]
    SettingsStartupSet(StartupSet),
    #[serde(rename = "settings.startup_add")]
    SettingsStartupAdd(AppId),
    #[serde(rename = "settings.startup_apps")]
    SettingsStartupApps,
    #[serde(rename = "settings.drivers")]
    SettingsDrivers,
    #[serde(rename = "settings.devices")]
    SettingsDevices,
    #[serde(rename = "settings.device_action")]
    SettingsDeviceAction(DeviceAction),
    #[serde(rename = "settings.system_info")]
    SettingsSystemInfo,
    #[serde(rename = "settings.check_updates")]
    SettingsCheckUpdates,
    #[serde(rename = "settings.open_releases")]
    SettingsOpenReleases,
    #[serde(rename = "settings.users")]
    SettingsUsers,
    #[serde(rename = "settings.user_create")]
    SettingsUserCreate(UserCreate),
    #[serde(rename = "settings.user_action")]
    SettingsUserAction(UserAction),
    #[serde(rename = "settings.driver_set")]
    SettingsDriverSet(DriverAction),

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
            Request::NotificationsGet
            | Request::NotificationsRead
            | Request::NotificationsDoNotDisturb(_)
            | Request::NotificationsDismiss(_)
            | Request::NotificationsClear
            | Request::NotificationsAction(_) => Some(Capability::Notifications),
            Request::CalendarList | Request::CalendarSave(_) => Some(Capability::Calendar),
            Request::PulseSnapshot => Some(Capability::Monitor),
            Request::DesktopGet => None,
            Request::PowerStatus => Some(Capability::PowerStatus),
            Request::DesktopFontSet(_)
            | Request::DesktopAppearanceSet(_)
            | Request::DesktopWallpaperSet(_)
            | Request::DesktopWallpaperChoose => Some(Capability::Appearance),
            Request::DesktopAddFile(_)
            | Request::DesktopChooseFiles
            | Request::DesktopRemove(_)
            | Request::DesktopOpen(_) => Some(Capability::DesktopFiles),
            Request::DesktopAddApp(_) => Some(Capability::Dock),
            Request::FileLocations
            | Request::FileBookmarks
            | Request::FileBookmarkSet(_)
            | Request::FileChooseFolder
            | Request::FileList(_)
            | Request::FileProperties(_)
            | Request::FileTrashList
            | Request::FileTrashRestore(_)
            | Request::FileOpen(_)
            | Request::FileMkdir(_)
            | Request::FileTransfer(_)
            | Request::FileTransferStart(_)
            | Request::FileTransferStatus(_)
            | Request::FileTransferCancel(_)
            | Request::FileTransfers
            | Request::FileThumbnail(_)
            | Request::FilePreview(_)
            | Request::FileWallpaper(_)
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
            | Request::PreferencesPageOpen(_)
            | Request::FinderOpen
            | Request::PulseOpen
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
            Request::WindowsList | Request::WindowsActivate(_) | Request::WindowsClose(_) => Some(Capability::Windows),
            Request::SettingsUsers
            | Request::SettingsUserCreate(_)
            | Request::SettingsUserAction(_)
            | Request::SettingsDevices
            | Request::SettingsDeviceAction(_)
            | Request::SettingsDrivers
            | Request::SettingsDriverSet(_) => Some(Capability::SystemAdministration),
            Request::SettingsStartup
            | Request::SettingsStartupSet(_)
            | Request::SettingsStartupAdd(_)
            | Request::SettingsStartupApps => Some(Capability::Startup),
            Request::SettingsSystemInfo | Request::SettingsCheckUpdates | Request::SettingsOpenReleases => {
                Some(Capability::Settings)
            }
            Request::SettingsPrimaryDisplay
            | Request::SettingsPrimaryDisplaySet(_)
            | Request::SettingsSearch
            | Request::SettingsSearchSet(_)
            | Request::SettingsDisplays
            | Request::SettingsIdle
            | Request::SettingsIdleSet(_)
            | Request::SettingsAudio
            | Request::SettingsAudioAction(_)
            | Request::SettingsDisplayApply(_)
            | Request::SettingsDisplayDecide(_)
            | Request::SettingsMonitorBrightness(_)
            | Request::SettingsWifiNetworks
            | Request::SettingsWifiScan
            | Request::SettingsWifiProfiles
            | Request::SettingsWifiConfig(_)
            | Request::SettingsWifiConfigSet(_)
            | Request::SettingsWifiProfileAction(_)
            | Request::SettingsWifiConnect(_)
            | Request::SettingsBluetoothDevices
            | Request::SettingsBluetoothScan
            | Request::SettingsBluetoothAction(_)
            | Request::SettingsBluetoothAudio
            | Request::SettingsBluetoothAudioSet(_)
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
    pub notifications: bool,
}

/// Events pushed from the host to a surface.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "event", content = "data")]
#[ts(export)]
pub enum Event {
    #[serde(rename = "notifications.changed")]
    NotificationsChanged(NotificationState),
    #[serde(rename = "pulse.visibility")]
    PulseVisibility(bool),
    #[serde(rename = "terminal.opened")]
    TerminalOpened,
    #[serde(rename = "terminal.opened_at")]
    TerminalOpenedAt(FilePath),
    #[serde(rename = "terminal.closed")]
    TerminalClosed,
    #[serde(rename = "finder.location")]
    FinderLocation(FilePath),
    #[serde(rename = "files.changed")]
    FilesChanged(FilePath),
    #[serde(rename = "preferences.wallpaper")]
    PreferencesWallpaper,
    #[serde(rename = "preferences.page")]
    PreferencesPage(PreferencesPageRequest),
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
pub const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;

pub fn parse_request(json: &str) -> Result<Request, ErrorBody> {
    if json.len() > MAX_REQUEST_BYTES {
        return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Request exceeds the 2 MiB limit."));
    }
    // Parser diagnostics must not echo user-supplied values, including secrets.
    let request: Request = serde_json::from_str(json).map_err(|e| {
        ErrorBody::new(
            ErrorCode::InvalidRequest,
            format!("Invalid request at line {}, column {}.", e.line(), e.column()),
        )
    })?;
    match &request {
        Request::SearchQuery(query) if query.query.len() > 4096 => {
            Err(ErrorBody::new(ErrorCode::InvalidRequest, "Search text exceeds 4096 bytes."))
        }
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
    fn new_desktop_workflows_retain_capability_boundaries() {
        let preview = Request::FilePreview(FilePath { path: "/tmp/readme".into() });
        assert_eq!(preview.required_capability(), Some(Capability::Files));
        assert!(SurfaceKind::Finder.has(Capability::Files));
        for surface in [SurfaceKind::Panel, SurfaceKind::Applications, SurfaceKind::Locker] {
            assert!(!surface.has(Capability::Files));
        }
        for request in [
            Request::SettingsSearch,
            Request::SettingsPrimaryDisplay,
            Request::SettingsSearchSet(SearchPreferences::default()),
        ] {
            assert_eq!(request.required_capability(), Some(Capability::Settings));
        }
        assert!(parse_request(r#"{"method":"search.activate","params":{"provider":"shell","item_id":"rm"}}"#).is_err());
        assert_eq!(
            Request::NotificationsDoNotDisturb(SetEnabled { enabled: true }).required_capability(),
            Some(Capability::Notifications)
        );
        assert!(!SurfaceKind::Preferences.has(Capability::Notifications));
        assert!(SurfaceKind::Topbar.has(Capability::Notifications));
        assert!(SurfaceKind::Options.has(Capability::Notifications));
    }

    #[test]
    fn startup_management_is_only_available_in_preferences() {
        for json in [
            r#"{"method":"settings.startup"}"#,
            r#"{"method":"settings.startup_apps"}"#,
            r#"{"method":"settings.startup_set","params":{"id":"mail.desktop","enabled":true}}"#,
            r#"{"method":"settings.startup_add","params":{"id":"mail.desktop"}}"#,
        ] {
            let request: Request = serde_json::from_str(json).unwrap();
            assert_eq!(request.required_capability(), Some(Capability::Startup));
            assert!(SurfaceKind::Preferences.has(Capability::Startup));
            for surface in [
                SurfaceKind::Options,
                SurfaceKind::Panel,
                SurfaceKind::Applications,
                SurfaceKind::Finder,
                SurfaceKind::Greeter,
            ] {
                assert!(!surface.has(Capability::Startup));
            }
        }
    }
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
    fn session_actions_are_available_to_desktop_controls() {
        for r in [
            Request::SessionLock,
            Request::SessionSignOut,
            Request::SessionSleep,
            Request::SessionRestart,
            Request::SessionShutdown,
        ] {
            let cap = r.required_capability().unwrap();
            assert!(SurfaceKind::Options.has(cap));
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
    fn wifi_configuration_requests_require_settings_capability() {
        for json in [
            r#"{"method":"settings.wifi_profiles"}"#,
            r#"{"method":"settings.wifi_config","params":{"uuid":"saved-id"}}"#,
            r#"{"method":"settings.wifi_config_set","params":{"uuid":"saved-id","autoconnect":true,"metered":"unknown","method":"auto","addresses":"","gateway":"","dns":"1.1.1.1","ignore_auto_dns":false}}"#,
            r#"{"method":"settings.wifi_profile_action","params":{"uuid":"saved-id","action":"forget"}}"#,
        ] {
            let request = parse_request(json).unwrap();
            assert_eq!(request.required_capability(), Some(Capability::Settings));
            assert!(SurfaceKind::Preferences.has(Capability::Settings));
            assert!(!SurfaceKind::Finder.has(Capability::Settings));
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub y: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub window_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub item_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SystemInfo {
    pub kernel: String,
    pub architecture: String,
    pub hostname: String,
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

/// Kernel modules for the currently running kernel; changes last until reboot/reload.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DriverInfo {
    pub name: String,
    pub path: String,
    pub loaded: bool,
    pub builtin: bool,
    pub users: u32,
    pub devices: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DriverAction {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct UserInfo {
    pub uid: u32,
    pub name: String,
    pub real_name: String,
    pub administrator: bool,
    pub locked: bool,
    pub current: bool,
    pub home: String,
}
#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct UserCreate {
    pub name: String,
    pub real_name: String,
    pub administrator: bool,
    pub password: String,
}
impl std::fmt::Debug for UserCreate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UserCreate(<redacted>)")
    }
}
#[derive(Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct UserAction {
    pub uid: u32,
    pub action: String,
    pub password: Option<String>,
}
impl std::fmt::Debug for UserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserAction").field("uid", &self.uid).field("action", &self.action).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct UpdateInfo {
    pub installed: String,
    pub latest: Option<String>,
    pub available: bool,
    pub message: String,
    pub published: Option<String>,
}

#[cfg(test)]
mod management_tests {
    use super::*;
    #[test]
    fn privileged_management_is_only_available_in_settings_and_secrets_are_redacted() {
        for request in [
            Request::SettingsDrivers,
            Request::SettingsUsers,
            Request::SettingsDriverSet(DriverAction { name: "test".into(), enabled: false }),
        ] {
            assert_eq!(request.required_capability(), Some(Capability::SystemAdministration));
        }
        assert!(SurfaceKind::Preferences.capabilities().contains(&Capability::SystemAdministration));
        for kind in [
            SurfaceKind::Options,
            SurfaceKind::Wallpaper,
            SurfaceKind::Applications,
            SurfaceKind::Finder,
            SurfaceKind::Terminal,
            SurfaceKind::Greeter,
            SurfaceKind::Locker,
        ] {
            assert!(!kind.capabilities().contains(&Capability::SystemAdministration));
        }
        let create = UserCreate {
            name: "tester".into(),
            real_name: "Tester".into(),
            administrator: false,
            password: "secret-password".into(),
        };
        let action = UserAction { uid: 1000, action: "password".into(), password: Some("secret-password".into()) };
        assert!(!format!("{create:?} {action:?}").contains("secret-password"));
    }
}

/// A device sample. Rates are bytes/second; unavailable readings are null.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ResourceSample {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub detail: String,
    pub percent: Option<f64>,
    pub used: Option<f64>,
    pub total: Option<f64>,
    pub read_rate: Option<f64>,
    pub write_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    pub state: String,
    /// Share of the entire machine, 0–100 (same scale as overall CPU).
    pub cpu: Option<f64>,
    pub memory: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct PulseSnapshot {
    pub resources: Vec<ResourceSample>,
    pub processes: Vec<ProcessSample>,
}

#[cfg(test)]
mod pulse_tests {
    use super::*;
    #[test]
    fn metrics_are_read_only_and_only_available_to_pulse() {
        assert_eq!(Request::PulseSnapshot.required_capability(), Some(Capability::Monitor));
        assert_eq!(SurfaceKind::Pulse.capabilities(), &[Capability::Monitor]);
        for kind in [
            SurfaceKind::Finder,
            SurfaceKind::Terminal,
            SurfaceKind::Applications,
            SurfaceKind::Preferences,
            SurfaceKind::Greeter,
            SurfaceKind::Locker,
        ] {
            assert!(!kind.has(Capability::Monitor));
        }
        assert_eq!(Request::PulseOpen.required_capability(), Some(Capability::Popovers));
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ManagedDevice {
    pub id: String,
    pub category: String,
    pub name: String,
    pub status: String,
    pub properties: Vec<(String, String)>,
    pub actions: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct DeviceAction {
    pub id: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct FileTransfer {
    pub path: String,
    pub destination: String,
    pub move_file: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct NotificationItem {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
    pub time: f64,
    pub unread: bool,
    pub actions: Vec<(String, String)>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct NotificationState {
    pub available: bool,
    pub do_not_disturb: bool,
    pub items: Vec<NotificationItem>,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct NotificationId {
    pub id: u32,
}
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct NotificationAction {
    pub id: u32,
    pub key: String,
}

#[cfg(test)]
mod desktop_controls_tests {
    use super::*;
    #[test]
    fn file_hardware_and_notification_controls_are_scoped_to_their_surfaces() {
        for request in [
            Request::SettingsDevices,
            Request::SettingsDeviceAction(DeviceAction { id: "net:wlan0".into(), action: "disconnect".into() }),
        ] {
            assert_eq!(request.required_capability(), Some(Capability::SystemAdministration));
            assert!(!SurfaceKind::Finder.has(request.required_capability().unwrap()));
        }
        for request in [
            Request::FileTransfer(FileTransfer {
                path: "/tmp/a".into(),
                destination: "/tmp/b".into(),
                move_file: false,
            }),
            Request::FileThumbnail(FilePath { path: "/tmp/a".into() }),
            Request::FileWallpaper(FilePath { path: "/tmp/a".into() }),
            Request::FileBookmarks,
        ] {
            assert_eq!(request.required_capability(), Some(Capability::Files));
            assert!(SurfaceKind::Finder.has(Capability::Files));
        }
        assert!(SurfaceKind::Notifications.has(Capability::Notifications));
        assert!(!SurfaceKind::Greeter.has(Capability::Notifications));
        assert!(!SurfaceKind::Locker.has(Capability::Notifications));
        assert_eq!(
            Request::DesktopFontSet(WallpaperSet { id: "Inter".into() }).required_capability(),
            Some(Capability::Appearance)
        );
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CalendarEvent {
    pub id: String,
    pub date: String,
    pub title: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct TransferId {
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TransferProgress {
    pub id: u32,
    pub bytes: f64,
    pub total: f64,
    pub entries: f64,
    pub done: bool,
    pub error: Option<String>,
}

#[cfg(test)]
mod reliability_tests {
    use super::*;
    #[test]
    fn malformed_and_oversized_messages_never_echo_input_or_panic() {
        let secret = "SECRET_MUST_NOT_APPEAR";
        for json in [
            "".to_owned(),
            "null".into(),
            "[]".into(),
            format!(r#"{{"method":"{secret}"}}"#),
            format!(
                r#"{{"method":"greeter.login","params":{{"username":"test","password":"{secret}","session":{{"{secret}":1}}}}}}"#
            ),
            "[".repeat(1000),
            "x".repeat(MAX_REQUEST_BYTES + 1),
        ] {
            let error = parse_request(&json).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidRequest);
            assert!(!error.message.contains(secret));
            assert!(error.message.len() < 256);
        }
    }
}
