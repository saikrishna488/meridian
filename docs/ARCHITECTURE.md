# Meridian Desktop — Architecture

Status: **Milestone 0 (prototype)**. This document describes the target
architecture and marks which parts exist today. The decision records under
[`docs/adr/`](adr/) explain the most important choices in more depth.

---

## 1. Goals and non-goals

Meridian is an original Linux desktop environment. It aims for the polish of
the best commercial desktops: consistent visuals, good typography, smooth
frame pacing, and simple interactions. It keeps Linux's flexibility and
hardware support.

Priorities, in order: responsiveness, visual consistency, typography, frame
pacing, low memory, low idle CPU, hardware acceleration, accessibility,
modularity, maintainability.

Non-goals for now:

- writing a compositor from scratch (planned later; see §3)
- a new application ecosystem (normal Wayland, X11, and Flatpak apps must
  keep working)
- an Electron-style "desktop as a web app"

---

## 2. The interaction model

([ADR-0005](adr/0005-top-bar.md) supersedes the earlier home-screen design in
ADR-0003.)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Applications │ Firefox  Terminal  Files                          Options │  ← top bar
├──────────────┴───────────┬──────────────────────────────┬───────────────┤
│ ┌──────────────────────┐ │                              │ Wi-Fi    [On] │
│ │ Search applications  │ │                              │ Bluetooth[Off]│
│ │ Firefox  Web Browser │ │       desktop (wallpaper)    │ Brightness 80%│
│ │ Files    File Manager│ │                              │ Volume     45%│
│ │ …                    │ │                              └───────────────┘
│ └──────────────────────┘ │                                              │
└──────────────────────────────────────────────────────────────────────────┘
```

- **Top bar.** "Applications" is on the left. Next to it is the list of
  **running windows**, by app name, with the focused one marked; clicking one
  focuses it. "Options" is at the far right. Nothing else is on the bar.
- **Applications menu.** A dropdown with a search field and a **text list**
  of every installed application (name + one-line description). Typing
  filters instantly (fuzzy over name, keywords, description). Up/Down move,
  Enter launches, and Escape clears the query, then closes.
- **Options menu.** **Wi-Fi** and **Bluetooth** as on/off switches, and
  **Brightness** and **Volume** as percentage sliders. They show and change
  the real system state. Unavailable controls are disabled, never faked.
- **Text only, no icons**, anywhere in the shell
  ([ADR-0004](adr/0004-text-only.md)).
- **Desktop.** Just the wallpaper.

Keyboard map (M0):

| Keys | Action |
|---|---|
| Super (tap) / Super+Space | Toggle Applications |
| typing in Applications | Search |
| ↑ ↓ / Tab / PgUp PgDn | Move selection |
| Enter | Launch selected |
| Esc | Clear query → close menu |
| Super+C | Toggle Options |
| Alt+Tab | Switch windows (labwc switcher, text list) |

## 3. Compositor strategy

**Decision:** start on an existing compositor. Talk to it only through
standard Wayland protocols, so it can be swapped without touching the shell.
([ADR-0002](adr/0002-compositor.md))

- **Default session compositor: [labwc](https://labwc.github.io/).** It is a
  wlroots-based *stacking* (floating-window) compositor. It is small and
  stable, has XWayland support, and has a built-in Alt+Tab switcher. It
  implements the protocols the shell needs: `wlr-layer-shell-unstable-v1`,
  `wlr-foreign-toplevel-management`, `ext-foreign-toplevel-list`,
  `ext-session-lock-v1`, `wlr-output-management`, `xdg-activation`,
  `wp-fractional-scale-v1`, and `wp-presentation-time`.
- **Also works on** any compositor with layer-shell (KWin, sway, Hyprland,
  niri, Wayfire, COSMIC). Development runs nested: `tools/dev-session.sh`
  opens labwc as a window inside your current desktop.
- **Protocols we depend on** (contract with the compositor):
  - M0: `wlr-layer-shell` (bar, menus, wallpaper),
    `wlr-foreign-toplevel-management` (running windows in the bar)
  - M1: `xdg-activation` (focus handoff on launch)
  - M3: `ext-session-lock-v1` (lock screen), `ext-idle-notify-v1`
  - M4: `ext-workspace-v1`, `ext-image-copy-capture` (screenshots and overview
    thumbnails), `wlr-output-management` (display settings)
- **Long term:** our own compositor on **Smithay** (Rust). It will add window
  open/close animations, a live window overview,
  consistent decorations, and tighter frame scheduling. The shell is already
  a pure protocol client, so the swap doesn't touch it.

Why not write the compositor now: a production compositor needs years of work
on input, DRM/KMS, multi-GPU, XWayland, and client quirks. The shell UX is
where we can show value first.

---

## 4. Shell architecture

```
┌──────────────────────────── user session (unprivileged) ─────────────────────────────┐
│                                                                                       │
│  ┌──────────────────── meridian-shell (Rust, one process) ────────────────────┐       │
│  │  Surface host ─ layer-shell surfaces                                       │       │
│  │    ├─ panel (top bar)          ┐                                           │       │
│  │    ├─ applications (menu)      │ WebViews: HTML/CSS/TypeScript             │       │
│  │    ├─ options (menu)           │ meridian:// only · no network · no files  │       │
│  │    └─ wallpaper (per output)   ┘                                           │       │
│  │  Bridge ── typed JSON requests/events + per-surface capability checks      │       │
│  │  Services (Rust)                                                           │       │
│  │    ├─ search: provider registry ─ apps provider (fuzzy)                    │       │
│  │    ├─ app catalog (XDG desktop entries)                                    │       │
│  │    ├─ windows (wlr-foreign-toplevel, own Wayland connection)               │       │
│  │    └─ settings: Wi-Fi (NM) · Bluetooth (BlueZ) · brightness (logind) ·     │       │
│  │                 volume (wpctl → PipeWire, M2: native)                      │       │
│  └──────────────────────────────┬─────────────────────────────────────────────┘       │
│                                 │ D-Bus (fixed calls only) / Wayland                  │
│   NetworkManager · BlueZ · logind · PipeWire/WirePlumber · compositor                 │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Surfaces

| Surface | Content | Layer | Keyboard | Where |
|---|---|---|---|---|
| `panel` | top bar | top, exclusive zone (36 px) | none | primary output |
| `applications` | search + app list (dropdown, left) | overlay, below the bar | exclusive while open | primary output |
| `options` | switches + sliders (dropdown, right) | overlay, below the bar | on-demand | primary output |
| `wallpaper` | wallpaper | background | none | every output |

**Menus sit below the bar.** Menu surfaces cover the output with exclusive
zone 0, so the compositor places them *under* the bar's reserved area. The
bar stays clickable while a menu is open (Applications → Options switches
directly). A transparent scrim in the menu surface catches outside clicks to
close. Menus are created hidden at startup and shown/hidden, so opening is
instant. The host tells the bar which menu is open, so it can highlight its
button.

**Running windows** come from `wlr-foreign-toplevel-management`, on a second,
tiny Wayland connection owned by the windows service and dispatched from the
GLib main loop. That keeps the shell's window list independent of GTK.
Window app ids are matched to desktop entries (id, `StartupWMClass`) to show
the application's name.

### 4.2 Web engine process model

- All WebViews share **one WebKit web process** (`related-view`), so memory
  stays near one browser tab.
- The network session is **ephemeral**: no cookies, no disk cache, no
  persistent storage.
- WebKit's bubblewrap/seccomp sandbox for the web process stays enabled.

### 4.3 Control interface

The shell is a `GApplication` with id `org.meridian.Shell`. Fixed,
argument-less actions are exported on the session bus. The compositor binds
keys to them:

```sh
gapplication action org.meridian.Shell toggle-applications
gapplication action org.meridian.Shell toggle-options
```

Nothing in this interface takes a command string.

### 4.4 Login screen

`meridian-greeter` ([ADR-0006](adr/0006-login-screen.md)) is a second binary
built from the same host library (`shell/src/lib.rs`). greetd runs it as the
`greetd` user inside a locked-down labwc. Its one surface (`greeter`) holds
only the `Login` capability: list sessions, log in, restart/shut down.
greetd does the authentication (PAM). The greeter relays the password over
greetd's socket and exits on success, and greetd then starts the chosen
session (Meridian, Plasma, …).

## 5. Search architecture

Search is a **provider registry** in the native layer, not UI-side filtering.
That way files, settings, calculator, and command providers can be added
later without changing the UI contract.

```
UI ── search.query {query, serial} ──▶ SearchService
                                       ├─ AppsProvider      (M0)
                                       ├─ SettingsProvider  (M4)
                                       ├─ CalculatorProvider(M4)
                                       └─ FilesProvider     (M4, async, via an indexer)
UI ◀── SearchResults {serial, sections:[{provider, items}]} ──
UI ── search.activate {provider, item_id} ──▶ provider decides what "activate" means
```

- **Items are opaque ids.** A result is `{provider, id, title, subtitle}`. Activating it sends the id back to the *same provider*, which
  resolves it against its own state. The UI never sends anything executable,
  so a command or settings provider can't be tricked into running
  UI-supplied strings.
- **Fast path:** the apps provider ranks about 100–1000 entries per
  keystroke in well under a millisecond. The bridge round trip is
  in-process. Results carry the query's `serial`, and the UI drops stale
  responses.
- **Async providers** (files, later) return their first section right away
  and push more as `search.updated` events keyed by serial. The protocol
  already has the serial for this.
- **Fuzzy matching** (`services/src/search/fuzzy.rs`) is a subsequence scorer
  with bonuses for prefix, word-start, and contiguous runs, and penalties for
  gaps. It is applied to name (weight 1.0), keywords and generic name (0.7),
  and description (0.4). It is ours (small, tested, no dependency) and can be
  swapped for `nucleo-matcher` if we need Unicode-aware matching at scale.
- **Empty query** returns all applications alphabetically. That is the full
  Applications list.

---

## 6. The native/web boundary

**Decision:** UI is written in HTML/CSS/TypeScript and rendered by
**WebKitGTK 6** inside a thin Rust host. GTK4 is used **only** as the
window/WebView container, for `gtk4-layer-shell`, and monitor enumeration.
No GTK widgets are used for UI.
([ADR-0001](adr/0001-web-ui-host.md))

Web content is treated as **untrusted by default**, as if an XSS bug in UI
code could happen at any time. A compromised surface can only do what its
capabilities allow:

1. **Content origin is locked.** WebViews load only `meridian://ui/…` and
   `meridian://assets/…`, served by a Rust handler from two fixed
   directories. Paths are normalized: no `..`, no absolute paths, and
   canonicalized paths must stay inside the root, so symlinks can't escape.
   Other navigation, new windows, downloads, and permission requests are
   denied.
2. **No network.** A Content-Security-Policy (`default-src meridian:`,
   `connect-src 'none'`) blocks fetch/XHR/WebSocket, and the network session
   is ephemeral.
3. **No file system.** There is no `file://` access, and the UI never sends
   a path. Because the shell is text-only, there is no icon endpoint; the
   only files served are the fixed UI and asset bundles.
4. **Typed messages only.** Requests must deserialize into the closed
   `Request` enum (`deny_unknown_fields`). Anything else is rejected.
5. **Per-surface capabilities.** Each surface gets a fixed capability set
   when it is created. Every request is checked against it before it runs.
   The wallpaper surface has none.
6. **No command strings.** Launching takes a desktop-entry **id**. The id is
   resolved against the installed catalog and launched by GIO's desktop-entry
   launcher, which parses `Exec` field codes per the XDG spec and never
   involves a shell. Launches get a proper `AppLaunchContext` (startup
   notification / xdg-activation token).
7. **No raw D-Bus.** Services make fixed calls and expose domain types
   (`SettingsState`, `WindowInfo`), never bus names or method names.

---

## 7. IPC mechanism

### 7.1 UI ⇄ host (in process, per WebView)

- **Requests (JS → Rust):** WebKit script message handler *with reply*. The
  JS side calls `window.webkit.messageHandlers.meridian.postMessage(json)`,
  which returns a `Promise`. Request:
  `{"method": "search.activate", "params": {"provider": "apps", "item_id": "org.gnome.Nautilus.desktop"}}`.
  Reply: `{"ok": true, "result": …}` or
  `{"ok": false, "error": {"code": "…", "message": "…"}}`.
- **Events (Rust → JS):** the host calls one fixed function,
  `window.__meridianDispatch(event)`, via `call_async_javascript_function`.
  The JSON is passed as a typed **argument**, never spliced into script
  source.
- **Schema:** Rust types in `protocol/` are the single source of truth.
  TypeScript types in `ui/src/lib/generated/` are generated by `ts-rs`
  (`cargo test -p meridian-protocol`). `tools/check.sh` fails if they are
  stale.

### 7.2 Host ⇄ system services (D-Bus)

- **M0:** services run in-process on the GLib main loop and use GIO's D-Bus
  client. That means no second event loop and no async runtime for a
  handful of read-only properties.
- **M2+:** stateful services (network, Bluetooth, audio, notifications, file
  indexing) move to a **`meridian-servicesd`** user daemon written with
  `zbus`, exposing typed `org.meridian.*` interfaces. The shell, the settings
  app, the lock screen, and a CLI become its clients, and a crash on one side
  doesn't take down the other. The service APIs in `services/` already have
  the shape that daemon will export, so only the transport changes.

---

## 8. Rendering approach

- **GPU end to end:** WebKitGTK renders with Skia on the GPU and hands
  DMA-BUFs to GTK4. GTK4 composites with GL or Vulkan and commits to the
  compositor. Hardware acceleration is forced on.
- **Refresh-rate correct:** GTK4's frame clock is driven by the compositor's
  `wl_surface.frame` callbacks on each output, so CSS animations and
  `requestAnimationFrame` run at the output's real rate (60/75/120/144+ Hz).
  UI rules:
  - Animate only `transform` and `opacity` (composited, no layout).
  - Use CSS transitions, or `requestAnimationFrame` with timestamp deltas.
    Never step animation with timers, and never assume 16.7 ms frames.
  - Motion and type tokens live in `ui/src/lib/tokens.css`. Durations are
    120–240 ms with decelerating curves. `prefers-reduced-motion` disables
    them.
- **Idle ≈ 0 CPU:** nothing animates or polls at rest. Window changes are
  pushed by the compositor, Wi-Fi/Bluetooth state by D-Bus signals, and
  app-catalog updates by `GAppInfoMonitor`. Brightness and volume are re-read
  only when Options opens.
- **Scrolling:** native WebKit scrolling, with kinetic touchpad scrolling
  from GTK. Keyboard navigation scrolls the selection into view with
  `scroll-behavior: smooth`.
- **Fractional scaling and resolution:** GTK4 implements
  `wp-fractional-scale-v1` and `wp-viewporter`, and WebKit renders text at
  the device scale, so it stays sharp at 125–200%. The list is a centered
  column sized with `min()`/`clamp()`, so it adapts from 1366×768 to 5K
  without breakpoints.
- **Typography:** Inter (variable) for UI text, falling back to Cantarell and
  Noto Sans. Tabular numerals for time and percentages. The type scale is in
  `tokens.css`.
- **Exit plan:** if WebKitGTK becomes limiting, move the host to WPE WebKit's
  Wayland platform (same engine, no GTK) or to Servo. Only
  `shell/src/surface.rs`, `bridge.rs`, and `scheme.rs` change.

---

## 9. System-service integrations

| Area | Service | Interface | Milestone |
|---|---|---|---|
| Applications | XDG desktop entries | GIO `AppInfo` + `AppInfoMonitor` | M0 ✅ |
| Session actions (logout, suspend, reboot) | systemd-logind | `org.freedesktop.login1` | M2 |
| Display brightness | logind | `Session.SetBrightness` (no root) | M2 |
| External monitor brightness | DDC/CI | `i2c-dev` with seat `uaccess` udev rule | M3 |
| Wi-Fi / networking | NetworkManager | `org.freedesktop.NetworkManager` | M2 |
| Bluetooth | BlueZ | `org.bluez` | M2 |
| Audio | PipeWire / WirePlumber | native API via `pipewire-rs` | M2 |
| Notifications | (we are the server) | `org.freedesktop.Notifications` | M3 |
| Screenshots / recording | xdg-desktop-portal + compositor | portal backend | M4 |

Privileged operations always go through the owning system service, which
authorizes with polkit. The shell never needs root.

---

## 10. Security model

The shell runs with the user's privileges and draws trusted UI, including
password entry on the lock screen later. Defenses are layered:

1. **No root, ever.** Privileged actions go to polkit-checked system daemons.
2. **Least-privileged web content** (§6): fixed origin, CSP, no network, no
   file access, closed schema, per-surface capabilities, WebKit sandbox on.
3. **No command strings anywhere.** Not from UI, search results, config, or
   D-Bus actions. Launching resolves desktop-entry ids against the catalog.
4. **Narrow D-Bus exposure.** The UI never sees D-Bus. Exported shell actions
   take no arguments.
5. **Platform security stays on.** WebKit sandboxing, SELinux, and Flatpak
   sandboxing are never disabled for convenience. Dev mode (`MERIDIAN_DEV=1`)
   only enables WebKit's inspector and context menu.
6. **Login screen** (M0): greetd runs PAM as root. The greeter is
   unprivileged, has a single capability, and never stores or logs the
   password.
7. **Lock screen (M3)** uses `ext-session-lock-v1`, so a crashed shell leaves
   the session locked. PAM authentication runs in a small separate helper,
   not in web content.

---

## 11. Application compatibility

Meridian is a shell, not a platform. Applications don't link against it.

- **Wayland apps** talk to the compositor directly.
- **X11 apps** run through XWayland, which labwc starts on demand.
- **Flatpak apps** export desktop entries under `…/flatpak/exports/share`,
  which is on `XDG_DATA_DIRS`. They show up in the Applications list and launch
  through their exported `Exec` line.
- Apps with `NoDisplay=true`, `Hidden=true`, or `OnlyShowIn`/`NotShowIn`
  excluding us are hidden, per the desktop-entry spec. We set
  `XDG_CURRENT_DESKTOP=Meridian`.
- **Portals:** M4 adds `meridian-portals.conf`, using the GTK backends until
  we write our own.

---

## 12. Multi-monitor strategy

- The host watches GDK's monitor list, which reflects `wl_output` hot-plug,
  and reconciles surfaces: every output gets `wallpaper`, and the
  **primary** output also gets the bar.
- In M0 the primary output is the first monitor GDK reports. M4 adds
  output-management configuration and a bar per output.
- Each surface renders at its own output's fractional scale and refresh rate
  (one frame clock per surface).
- Menus open on the primary output, under its bar.

---

## 13. Repository layout

```
Cargo.toml          Rust workspace
shell/              meridian-shell + meridian-greeter: surface host, bridge, URI scheme, greetd client
protocol/           meridian-protocol: IPC types (Rust source of truth → generated TS)
services/           meridian-services: app catalog, search + fuzzy matcher, windows, settings
ui/                 TypeScript/HTML/CSS for each surface
  src/lib/          bridge client, generated protocol types, design tokens
  src/surfaces/     panel, applications, options, wallpaper, greeter
assets/             wallpapers
session/            labwc configs (session + login screen), session entry, launcher script
tools/              dev environment, nested sessions, fake greetd, install script, checks
docs/               this document, ADRs, development guide
```

Changes from the originally suggested layout:

- **No `/native`.** Rust code is split by responsibility (`shell`,
  `services`, `protocol`) as workspace crates.
- **`/session` added** for compositor configuration and session entry points.
- **No `/apps` or top-level `/tests` yet.** Tests live in each crate
  (`cargo test`), and UI type-checking runs through `tsc`. These directories
  will be added with the first app and the first end-to-end harness. We don't
  create empty placeholders.

---

## 14. Development roadmap

| Milestone | Scope |
|---|---|
| **M0 — Prototype** | Top bar (Applications, running windows, Options), Applications menu (search + text list of real installed apps, keyboard/mouse, launching), Options (real Wi-Fi/Bluetooth switches, brightness/volume sliders), login screen on greetd, installer, fuzzy search provider, typed bridge with capabilities, nested dev session with host apps |
| **M1 — Windows** | foreign-toplevel: Meridian window overview/switcher replaces labwc's Alt+Tab, focus/minimize from the bar, `xdg-activation` |
| **M2 — Controls** | `meridian-servicesd` (zbus); native PipeWire volume with change notifications; Wi-Fi network list, Bluetooth devices, power menu (logind), media-key OSD, clock |
| **M3 — Session** | notifications (server + center), lock screen, idle/DPMS, DDC/CI brightness, calendar |
| **M4 — Search & polish** | settings/calculator/file providers, workspaces, screenshot UI, shortcut config, display settings, accessibility audit |
| **M5+** | Smithay compositor (window animations, live overview), settings app, file manager, terminal, greeter, ISO/installer, SDK |
