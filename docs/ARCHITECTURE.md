Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# Meridian Desktop — Architecture

Status: **installable desktop session prototype; not ready as a sole daily-use desktop**.
It targets Debian testing and compatible derivatives. Stability, hardware,
installation, and workflow qualification remain open. This document describes
the current implementation. Older milestone labels in [decision records](adr/)
describe development history; they are not a reliable list of what is
implemented today.

## 1. Overview

Meridian is a Linux desktop shell built with Rust and React/TypeScript.
WebKitGTK renders the web interface, GTK4 hosts its windows, and labwc manages
application windows and composites the desktop. Linux and existing system
services continue to provide hardware support, networking, audio, and sessions.

The workspace contains three Rust crates: `shell`, `services`, and `protocol`.
The services crate is linked into the shell; there is no separate
`meridian-servicesd` executable today. Normal applications run independently
and communicate with labwc directly, or through XWayland for X11 applications.

Meridian provides a usable desktop session, but it is still a prototype. Some
planned Settings and system-management areas are not implemented, and hardware
behavior depends on the Linux services and devices available on the machine. The project
has not completed its stability, hardware, clean-install, rollback, and extended
daily-use qualification; see [Phase 1](PHASE1.md) and [Phase 3](PHASE3.md).

## 2. Current desktop

- **Top bar and dock:** Meridian menu, focused-window actions, Control Center,
  notifications, battery and clock; a centered, reorderable dock holds the
  launcher, pinned and running apps, and Trash.
- **Applications:** installed-app search and launch, running-window activation,
  dock pinning, desktop shortcuts, and reviewed uninstall plans for supported
  package managers.
- **Desktop:** bundled or custom wallpaper, application shortcuts, live files
  from the XDG Desktop folder, context actions, and no-overwrite file drops.
- **Settings:** dark-only appearance and font selection; wallpaper, startup
  apps, applications, search, Wi-Fi, Bluetooth, displays, sound, power and
  lock, general system information, device management, and local users.
- **Control Center and notifications:** connectivity, display brightness,
  volume, Do Not Disturb, session power actions, and in-memory notification
  history. Meridian consumes the standard notification D-Bus service; it does
  not provide a toast daemon.
- **Utilities:** Pulse reads system and process metrics. Meridian's Calendar
  and Clock entries launch the installed GNOME Calendar and GNOME Clocks apps.
- **External desktop apps:** GNOME Files handles file browsing and Kitty
  provides the interactive terminal. They are launched as independent host
  applications; the older embedded Finder and terminal interfaces are not in
  the production UI bundle.
- **Session actions:** lock, sleep, restart, shutdown, and sign out. Automatic
  locking uses Meridian's native session-lock surface; `swayidle` and `wlopm`
  handle the inactivity and display-standby timers. PipeWire media playback
  inhibits those timers.

System sharing is not implemented. Some settings and device actions depend on
the services and hardware present on the host.

See the [README](../README.md) for shortcuts and usage.

## 3. Session and compositor

`session/meridian.desktop` registers the login-session entry.
`session/meridian-session` sets the desktop environment variables and launches
labwc with `meridian-shell` as its startup command. This permits application
windows to survive a shell crash while labwc remains running.

The compositor provides window placement, focus, Alt+Tab, output composition,
and XWayland support. Meridian uses layer-shell for desktop surfaces,
foreign-toplevel management for the window list/activation, and output
management for display configuration. Meridian owns the lock surface and
authenticates through its `meridian-lock` PAM service.

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
| Panel | Centered dock: launcher, pinned/running apps, and separated Trash | Bottom-anchored transparent top layer, reserves 88 logical pixels |
| Topbar | Meridian menu, focused app menu with Quit, system controls and clock | Top-anchored full-width top layer, reserves 32 logical pixels |
| Applications / Options / DesktopMenu / Notifications | Launcher, Control Center, menus, notification history | Overlay layer |
| Preferences / Pulse | Settings and Meridian's system monitor | Ordinary GTK windows |
| Locker | Custom Meridian password prompt with clock and current wallpaper | Compositor-enforced `ext-session-lock-v1` surface on every monitor |
| Greeter | Login screen in a separate executable | Overlay in the greeter session |

Panel uses `surfaces/panel`; its 88-pixel surface includes space for the floating dock shadow. Topbar uses `surfaces/topbar`. The launcher opens above the dock; Meridian and application menus, Control Center, and notifications open below the top bar. Quit requests graceful closure through the compositor, allowing applications to prompt for unsaved work.

The host watches monitor changes. Each output gets a wallpaper; the first
monitor reported by GDK receives the dock and top bar and is used for menus.
This is an implementation choice, not a user-configurable primary-output policy.

## 5. UI and rendering

Each surface loads locally bundled HTML, CSS, and React/TypeScript through
`meridian://ui/surfaces/<name>/index.html`. Esbuild produces the JavaScript
bundles; Node.js is a build dependency, not the UI runtime.

Each WebView uses a separate WebKit context and ephemeral network session so a
renderer failure can recover independently. GTK also supplies native window
controls, file pickers, and other integration; the interface is not exclusively
web pixels. See [ADR-0008](adr/0008-component-recovery.md).

Hardware acceleration is requested for most WebViews. Actual rendering,
frame pacing, and memory usage depend on WebKit, drivers, hardware, and workload;
the repository does not establish a fixed memory budget or zero-idle-CPU result.
Pulse and Clock pause periodic updates when their windows are hidden.

Meridian's UI is dark-only. The selected system font is saved by the host and
applied to Meridian, GTK, and supported desktop font settings. External apps
can use their own appearance and font preferences.

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
Optional container development can resolve host entries and launch through
`flatpak-spawn --host gio launch`.

Search runs in Rust through installed-app and Settings-page providers, with an
optional bounded arithmetic calculator. The fuzzy scorer considers names,
keywords, generic names, and descriptions. Results carry a serial so the UI can
discard stale responses. Additional providers, including recent files, remain
future work.

Window information comes from a separate Wayland connection integrated with
the GLib loop. Application identifiers are matched to catalog entries for
display names and icons. Clicking a window action asks the compositor to
activate that window.

## 8. System integrations

| Feature | Current implementation |
|---|---|
| Wi-Fi toggle | NetworkManager through D-Bus |
| Wi-Fi status/discovery/join | NetworkManager and `nmcli`; page load reads cached state, explicit New connection triggers a rescan, passwords are supplied on stdin |
| Bluetooth | BlueZ through D-Bus; paired devices load without discovery, Pair new device starts a temporary scan and pairing uses a temporary agent |
| Bluetooth audio codecs | PipeWire/WirePlumber negotiates codecs supported by the device and installed system components; Settings documents tradeoffs but does not force codec selection |
| Media idle inhibition | `tools/build-release.sh` builds the pinned `wayland-pipewire-idle-inhibit` helper and `tools/install.sh` installs it; it holds the Wayland idle inhibitor during PipeWire playback while `swayidle` keeps normal lock and standby timers |
| Volume | WirePlumber's `wpctl`, with rapid slider updates coalesced |
| Built-in backlight | sysfs discovery and logind brightness control |
| Monitor layout | Wayland output-management test/apply |
| Sound | PipeWire/WirePlumber tools, including a temporary virtual output when supported |
| Notification history | `org.freedesktop.Notifications`; in-memory session history, no toast service |
| Local accounts | AccountsService and PolicyKit-mediated administrative actions |
| Printers and storage | CUPS controls, UDisks actions, GNOME Disks for partition/filesystem work |
| Idle lock and display standby | `swayidle` calls Meridian's `lock-session` action; `wlopm` powers down outputs after lock |
| Suspend/sign out/restart/shutdown | systemd-logind |
| Lock authentication | Meridian's native lock UI verifies the current account through PAM (`/etc/pam.d/meridian-lock`) |
| Login | Separate `meridian-greeter` communicating with greetd |
| App removal | Exact Flatpak ref, Snap name, or dpkg-owned desktop entry; privileged removals use `pkexec` |

Monitor changes revert after 15 seconds unless kept. Accepted layouts are saved
for later sessions. External monitor brightness currently directs users to the
monitor's hardware controls; DDC/CI control is not implemented.

Some operations live in `shell/src/settings_devices.rs` and
`shell/src/bluetooth_agent.rs`, rather than the services crate.

## 9. Files, applications, and persistence

The production launcher opens GNOME Files for directory browsing and Kitty for
terminal sessions. Meridian's own desktop file operations use GIO: desktop
items are watched for external changes, drops and transfers avoid overwriting
existing names, and deleting real files uses the system Trash. File operations
run with the signed-in user's permissions.

The older Finder and PTY terminal implementations remain in the source tree for
integration coverage and compatibility, but their React surfaces are excluded
from the production UI build. The current session delegates interactive shell
behavior to Kitty.

Preferences use the user's configuration directory, normally
`~/.config/meridian/`: `desktop.json` stores appearance, wallpaper, and
shortcuts; `dock-layout.json` stores dock ordering; `displays.json` stores
accepted monitor layouts. Custom wallpapers are copied into the configuration
directory so moving the original does not break the selection.

## 10. Trust boundaries

The shell runs as the logged-in user. Privileged system operations are mediated
by the owning services or authentication helpers.

- Requests must deserialize into the closed protocol and pass capability checks.
  Settings, app management, notifications, Calendar, desktop files, Greeter, and
  Locker have scoped capabilities. Legacy Finder and PTY terminal capabilities
  do not put those UI surfaces in the current production bundle.
- Web content uses local `meridian://` resources. Navigation and permission
  requests are restricted, and the WebKit network session is ephemeral.
  The exact repository link is opened externally by the host.
- UI/assets paths are confined to their roots. Additional routes serve the
  chosen custom wallpaper, catalog-resolved app icons, and theme icons.
- Direct browser file access is disabled. Authorized native bridge requests
  deliberately allow the desktop's scoped file operations.
- App launches resolve installed catalog IDs. The terminal shortcut launches
  the installed Kitty terminal application.
- Lock screens are owned by Meridian through `ext-session-lock-v1`; lock
  authentication is handled by the shell's PAM worker, not a separate locker.

These are implementation boundaries, not a claim that the prototype has
completed a security audit.

## 11. Development and verification

See [DEVELOPMENT.md](DEVELOPMENT.md) for Debian setup and nested sessions.
`tools/check.sh` checks Rust formatting, clippy, Rust tests, generated protocol
types, and the UI type-check/build. Run `cd ui && npm test` separately for UI
interaction tests with a mocked host. Those tests do not replace live validation
of graphics, hardware, PAM, or compositor behavior.

## 12. Current limits and future work

System sharing is not implemented. Some planned Settings controls remain
unfinished or depend on host services and hardware. Meridian's top-bar
menus are shell menus; they do not import or control arbitrary applications'
menu bars. Meridian does not provide its own package repository, automatic
distribution upgrades, notification toast daemon, or desktop ISO.

A separate services daemon, direct PipeWire APIs, notification toasts,
external-monitor DDC/CI brightness, additional search providers, a custom
compositor, and an ISO/SDK remain future directions from the older roadmap.
The current session already uses `swayidle` for idle locking and the standard
notification D-Bus API for history.

## 13. Repository layout

| Path | Responsibility |
|---|---|
| `protocol/src/lib.rs` | Message types, capabilities, validation, JSON replies, generated TS |
| `services/src/` | App catalog/search, window tracking, settings, output management |
| `shell/src/shell.rs` | Shared shell state, authorization/lock gating and request routing |
| `shell/src/shell/` | Feature handlers and lazy surface construction |
| `shell/src/surface.rs` | GTK/WebKit windows, layer placement, rendering settings |
| `shell/src/bridge.rs`, `scheme.rs` | Message transport and local resource loading |
| `shell/src/files.rs`, `desktop.rs`, `calendar.rs`, `pulse.rs` | File operations, desktop persistence, calendar data and metrics |
| `shell/src/settings_devices.rs`, `bluetooth_agent.rs` | Device configuration and pairing |
| `shell/src/greeter.rs`, `greetd.rs`, `lock_auth.rs`, `power.rs` | Login, lock, and session actions |
| `ui/src/surfaces/` | React interfaces for each surface |
| `ui/src/components/`, `lib/` | Shared components, hooks, bridge, types, design tokens |
| `assets/` | Icons, wallpapers, and labwc decoration theme |
| `session/` | Session entry, launcher, labwc configuration, PAM configuration |
| `tools/` | Build, development, install, codec, and check scripts |
| `docs/adr/` | Historical design decisions and implementation updates |

Desktop file drops use GIO moves without overwrite and preserve source files on name collisions. A GIO directory monitor publishes `desktop.changed` when the Desktop folder changes outside Meridian. Real files use system Trash; legacy shortcuts retain removal-only behavior.


## Alpha 0.2 reliability qualification

See [Phase 1](PHASE1.md) for current recovery behavior, capability audit, transfer
failure semantics, CI smoke tests and mandatory hardware/endurance release gates.
The session supervisor restarts only the shell; labwc remains independent.
Discarded surfaces explicitly destroy their GTK windows. Meridian creates its
own compositor-enforced lock surface on each monitor. File transfers stage data before no-overwrite publication;
cross-device held sources have explicit recovery manifests.


Performance ownership: `Shell` owns application surface slots and reuses each
window after first construction. `WebEnv` owns the shared scheme handler and its
bounded icon caches, while renderer contexts and storage sessions stay isolated.
Theme subscriptions hold weak references and disconnect when `WebEnv` drops.
Production ESM chunks share source modules across entries; independent WebKit
processes still allocate their own JavaScript heaps. Bundle-size reduction must
not be described as equivalent physical-memory reduction. See [Phase 2](PHASE2.md)
for measurement boundaries and remaining hardware qualification.


Settings search uses a native allowlist of typed preferences pages. Calculator
search parses bounded arithmetic and recomputes answers before clipboard writes;
neither provider can execute arbitrary input. Search preferences are cached by
`Shell` and stored with the versioned configuration helper. File preview and
tabbed Orbit behavior documented in older milestone notes refers to retained
legacy source and is not included in the production UI bundle. The primary-output
preference is stored in desktop configuration and only controls Meridian's
surfaces. Notification Do Not Disturb stores a boolean preference; notification
bodies remain session-only. See [Phase 3](PHASE3.md) for completed workflow
work and remaining beta qualification.
