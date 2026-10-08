# Meridian Desktop — Architecture

Status: **working prototype**. This document describes the current implementation.
Older milestone labels in [decision records](adr/) describe development history;
they are not a reliable list of what is implemented today.

## 1. Overview

Meridian is a Linux desktop shell built with Rust and React/TypeScript.
WebKitGTK renders the web interface, GTK4 hosts its windows, and labwc manages
application windows and composites the desktop. Linux and existing system
services continue to provide hardware support, networking, audio, and sessions.

The workspace contains three Rust crates: `shell`, `services`, and `protocol`.
The services crate is linked into the shell; there is no separate
`meridian-servicesd` executable today. Normal applications run independently
and communicate with labwc directly, or through XWayland for X11 applications.

## 2. Current desktop

- **Menu bar:** Meridian menu, active application name, File/Edit/View/Window/Help,
  Control Center, and clock. These are Meridian menus, not imported application
  menus; Edit commands are currently disabled.
- **Dock:** icon-based launcher, pinned/running applications, and Trash.
  Items can be reordered. Opening Applications or a fullscreen window hides
  the desktop dock.
- **Applications:** searchable installed applications, icons, launch and
  window activation actions, pinning, shortcuts, and reviewed removal.
- **Desktop:** bundled/custom wallpapers and application/file/folder shortcuts.
  Removing a shortcut preserves its target.
- **Control Center:** connectivity, appearance, brightness, and volume controls.
- **Settings:** working wallpaper, light/dark appearance, Wi-Fi, Bluetooth,
  and display configuration. Sharing is a UI prototype; some other categories
  and controls remain placeholders.
- **Finder:** real filesystem browsing, navigation, filtering, folder creation,
  rename, Trash, and an Applications view.
- **Terminal:** xterm.js connected to real interactive shells through PTYs.
- **Session actions:** Lock, Sleep, Restart, Shut Down, and Sign out are in
  the Meridian menu. Restart, Shut Down, and Sign out require confirmation.

See the [README](../README.md) for shortcuts and usage.

## 3. Session and compositor

`session/meridian.desktop` registers the login-session entry.
`session/meridian-session` sets the desktop environment variables and launches
labwc with `meridian-shell` as its startup command. This permits application
windows to survive a shell crash while labwc remains running.

The compositor provides window placement, focus, Alt+Tab, output composition,
and XWayland support. Meridian uses layer-shell for desktop surfaces,
foreign-toplevel management for the window list/activation, output management
for display configuration, and session-lock for locking.

labwc is the supplied configuration. Other compositors need the corresponding
protocols; layer-shell support alone does not guarantee full functionality.
The window list is disabled if its protocol connection is unavailable.

## 4. Native host and surfaces

`shell/src/bin/meridian-shell.rs` creates the GTK application.
`Shell::start` in `shell/src/shell.rs` initializes the catalog, search, settings,
window tracking, saved display layout, and surfaces. GTK/GLib coordinates events;
potentially blocking work is also dispatched to worker threads.

| Surface kind | Role | Placement |
|---|---|---|
| Wallpaper | Background and shortcuts | Background layer, every output |
| Topbar | Menu bar | Top layer, reserves 32 logical pixels |
| Panel | Dock (the internal name is still Panel) | Bottom-anchored top layer, reserves 70 logical pixels |
| Applications / Options / DesktopMenu | Launcher and menus | Overlay layer |
| Preferences / Finder / Terminal | Built-in application windows | Ordinary GTK windows |
| Locker | Password prompt | Session-lock surface per monitor |
| Greeter | Login screen in a separate executable | Overlay in the greeter session |

Topbar and Panel share the `surfaces/panel` frontend entry point. The dock
surface is 128 logical pixels tall to allow hover effects, but reserves only
70 pixels for its visible dock and bottom gap.

The host watches monitor changes. Each output gets a wallpaper; the first
monitor reported by GDK receives the topbar and dock and is used for menus.
This is an implementation choice, not a user-configurable primary-output policy.

## 5. UI and rendering

Each surface loads locally bundled HTML, CSS, and React/TypeScript through
`meridian://ui/surfaces/<name>/index.html`. Esbuild produces the JavaScript
bundles; Node.js is a build dependency, not the UI runtime.

WebViews use a shared WebKit context, an ephemeral network session, and related
views to share a web process. GTK also supplies native window controls,
file pickers, and other integration; the interface is not exclusively web pixels.

Hardware acceleration is requested for most WebViews. Terminal explicitly
disables it to avoid stale text rendering when maximized. Actual rendering,
frame pacing, and memory usage depend on WebKit, drivers, hardware, and workload;
the repository does not establish a fixed memory budget or zero-idle-CPU result.
Terminal polls bounded output chunks, and the clock also schedules updates.

Appearance is saved by the host and propagated through `desktop.changed`.
The shared appearance hook applies document theme/color-scheme state.
GTK preferences, desktop color-scheme settings, and an available KDE palette
tool are also updated. External applications can retain their own overrides.
Terminal text remains black on white while its surrounding UI follows appearance.

## 6. JavaScript–Rust protocol

`protocol/src/lib.rs` is the shared contract: requests, events, data structures,
error replies, input validation, and per-surface capabilities.
`ts-rs` generates the TypeScript definitions under `ui/src/lib/generated/`.

1. The UI calls `request()` in `ui/src/lib/bridge.ts`.
2. WebKit's `meridian` script-message handler receives a JSON string.
3. `shell/src/bridge.rs` parses it and checks its required capability.
4. `Shell::dispatch` routes it to the relevant implementation.
5. A JSON success/error reply resolves the JavaScript promise.

For example:

```json
{"method":"settings.set_volume","params":{"percent":50}}
```

The dispatcher calls the settings service, which invokes `wpctl` with fixed
arguments. Rust can also push typed events through
`window.__meridianDispatch`; event JSON is passed as an argument rather than
inserted into JavaScript source. The `shell.hello` handshake marks a surface
ready for events.

Fixed GApplication actions such as `toggle-applications`, `toggle-options`,
and `lock-session` expose keyboard-triggered actions over the session bus.

## 7. Applications and search

`services/src/apps.rs` builds a catalog from XDG desktop entries using GIO,
and monitors application changes. Installed sessions launch through GIO;
toolbox development can resolve host entries and launch through
`flatpak-spawn --host gio launch`.

Search runs in Rust through the apps provider. The fuzzy scorer considers
names, keywords, generic names, and descriptions. Results carry a serial so
the UI can discard stale responses. Built-in Settings, Finder, and Terminal
entries are handled by the shell. Additional search providers remain future work.

Window information comes from a separate Wayland connection integrated with
the GLib loop. Application identifiers are matched to catalog entries for
display names and icons. Clicking a window action asks the compositor to
activate that window.

## 8. System integrations

| Feature | Current implementation |
|---|---|
| Wi-Fi toggle | NetworkManager through D-Bus |
| Wi-Fi discovery/join | `nmcli`; passwords supplied on stdin |
| Bluetooth | BlueZ through D-Bus, with a temporary pairing agent |
| Volume | WirePlumber's `wpctl`, with rapid slider updates coalesced |
| Built-in backlight | sysfs discovery and logind brightness control |
| Monitor layout | Wayland output-management test/apply |
| Suspend/sign out/restart/shutdown | systemd-logind |
| Lock authentication | PAM via `meridian-lock` |
| Login | Separate `meridian-greeter` communicating with greetd |
| App removal | Resolved Flatpak ref or RPM package; RPM removal uses `pkexec dnf` |

Monitor changes revert after 15 seconds unless kept. Accepted layouts are saved
for later sessions. External monitor brightness currently directs users to the
monitor's hardware controls; DDC/CI control is not implemented.

Some operations live in `shell/src/settings_devices.rs` and
`shell/src/bluetooth_agent.rs`, rather than the services crate.

## 9. Finder, Terminal, and persistence

Finder's bridge requests reach `shell/src/files.rs`, which operates on the
user's real filesystem. Rename avoids overwriting another item; deletion uses
the system Trash. File operations are subject to the user's OS permissions.

`shell/src/terminal.rs` creates PTYs and starts the user's interactive shell.
The frontend renders output with bundled xterm.js. Input, bounded output reads,
resize, and close travel through the typed bridge. Closing a tab ends its shell
and foreground job; detached jobs can survive. In toolbox development the shell
runs inside the toolbox, with the shared home directory.

Preferences use the user's configuration directory, normally
`~/.config/meridian/`: `desktop.json` stores appearance, wallpaper, and
shortcuts; `dock-layout.json` stores dock ordering; `displays.json` stores
accepted monitor layouts. Custom wallpapers are copied into the configuration
directory so moving the original does not break the selection.

## 10. Trust boundaries

The shell runs as the logged-in user. Privileged system operations are mediated
by the owning services or authentication helpers.

- Requests must deserialize into the closed protocol and pass capability checks.
  Finder has file-operation capabilities; Terminal has terminal capabilities;
  Greeter has login and Locker has unlock capabilities.
- Web content uses local `meridian://` resources. Navigation and permission
  requests are restricted, and the WebKit network session is ephemeral.
  The exact repository link is opened externally by the host.
- UI/assets paths are confined to their roots. Additional routes serve the
  chosen custom wallpaper, catalog-resolved app icons, and theme icons.
- Direct browser file access is disabled. Authorized native bridge requests
  still deliberately allow filesystem operations and terminal input.
- App launches resolve installed catalog IDs. The Terminal intentionally
  executes commands entered by the user through a real shell.
- The lock uses the compositor's session-lock protocol; PAM verification runs
  off the GTK main thread.

These are implementation boundaries, not a claim that the prototype has
completed a security audit.

## 11. Development and verification

See [DEVELOPMENT.md](DEVELOPMENT.md) for toolbox setup and nested sessions.
`tools/check.sh` checks Rust formatting, clippy, Rust tests, generated protocol
types, and the UI type-check/build. Run `cd ui && npm test` separately for UI
interaction tests with a mocked host. Those tests do not replace live validation
of graphics, hardware, PAM, or compositor behavior.

## 12. Current limits and future work

Sharing is a UI prototype and does not start services. Settings categories such
as Sound and Privacy & Security contain placeholders, although Control Center
already provides working volume controls. Global menu items do not integrate
with arbitrary applications' menus.

A separate services daemon, native PipeWire integration, notification server,
idle locking, DDC/CI brightness, additional search providers, a custom
compositor, and an ISO/SDK remain future directions from the older roadmap.
They are not prerequisites for describing the features already implemented.

## 13. Repository layout

| Path | Responsibility |
|---|---|
| `protocol/src/lib.rs` | Message types, capabilities, validation, JSON replies, generated TS |
| `services/src/` | App catalog/search, window tracking, settings, output management |
| `shell/src/shell.rs` | Surface lifecycle and request dispatcher |
| `shell/src/surface.rs` | GTK/WebKit windows, layer placement, rendering settings |
| `shell/src/bridge.rs`, `scheme.rs` | Message transport and local resource loading |
| `shell/src/files.rs`, `terminal.rs`, `desktop.rs` | Built-in app operations and desktop persistence |
| `shell/src/settings_devices.rs`, `bluetooth_agent.rs` | Device configuration and pairing |
| `shell/src/greeter.rs`, `greetd.rs`, `lock_auth.rs`, `power.rs` | Login, lock, and session actions |
| `ui/src/surfaces/` | React interfaces for each surface |
| `ui/src/components/`, `lib/` | Shared components, hooks, bridge, types, design tokens |
| `assets/` | Icons, wallpapers, and labwc decoration theme |
| `session/` | Session entry, launcher, labwc configuration, PAM configuration |
| `tools/` | Build, development, install, codec, and check scripts |
| `docs/adr/` | Historical design decisions and implementation updates |
