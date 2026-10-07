# Developing Meridian

## Environment

The build dependencies live in a Fedora **toolbox** container
(`meridian-dev`), so nothing is installed on the host:

```sh
tools/dev-env.sh
```

It installs: Rust (cargo, rustfmt, clippy), Node.js + npm (TypeScript),
`gtk4-devel`, `webkitgtk6.0-devel`, `gtk4-layer-shell-devel` (≥ 1.2.0, for
the lock screen's session-lock surfaces too), `pam-devel` (lock screen
password check), `clang-devel` (libclang: `gtk4-session-lock-sys` generates
its bindings at build time), labwc, Xwayland, Inter fonts, foot (a terminal
to launch), grim (screenshots), and WirePlumber (`wpctl`, used for volume).

The `tools/*.sh` scripts re-run themselves inside the toolbox
automatically. To build on the host instead, install the same packages with
`sudo dnf install …` and set `MERIDIAN_NO_TOOLBOX=1`.

## Build

```sh
(cd ui && npm ci && npm run build)   # → ui/dist
cargo build -p meridian-shell        # → target/debug/meridian-shell
```

The UI uses React and TypeScript. Each surface has an HTML entry point and a
React component in `ui/src/surfaces/<name>/<name>.tsx`; shared components live
in `ui/src/components`. The typed host bridge remains in `ui/src/lib/bridge.ts`.
Esbuild bundles React locally into each surface's JavaScript file, preserving
the `meridian://ui/surfaces/<name>/index.html` URLs and existing CSS.

Use `applyAppearance()` and `useAppearance()` from `ui/src/lib/appearance.ts`
for system-app appearance behavior. The shared mount layer reads the persisted
mode and applies it to the document root, so CSS should use `[data-theme="dark"]`
or theme variables rather than maintaining private appearance state. React
surfaces that need to repaint non-CSS content can subscribe with
`subscribeAppearance()` or use the hook. Terminal emulator output intentionally
uses a fixed black-on-white palette; its surrounding window controls remain
theme-aware.

- `cd ui && npm run check` checks TypeScript, including JSX.
- `cd ui && npm test` builds and tests surface interactions with a mocked host.
- `cd ui && npm run watch` rebuilds JavaScript and copies HTML/CSS on changes.
  Reload the WebView through the inspector, or restart the nested session, to
  see changes. Watch builds include React's development diagnostics.


## Run

```sh
tools/dev-session.sh            # debug build, MERIDIAN_DEV=1
tools/dev-session.sh --release
```

This opens a **nested labwc** window (1280×720, resizable) running
`meridian-shell`. Closing the window ends the session.
Each nested session uses its own D-Bus session bus so an existing shell
cannot intercept startup or keyboard shortcuts.

- Dev mode (`MERIDIAN_DEV=1`) enables WebKit's inspector: right-click →
  Inspect Element.
- Logs: `RUST_LOG=debug tools/dev-session.sh`.
- Drive it from a terminal launched inside the nested desktop (so it
  inherits that desktop's private D-Bus session bus):
  ```sh
  export WAYLAND_DISPLAY=wayland-1           # the nested compositor
  gapplication action org.meridian.Shell toggle-applications
  gapplication action org.meridian.Shell toggle-options
  gapplication action org.meridian.Shell lock-session
  grim shot.png                              # screenshot
  ```
- **Your installed apps in the toolbox.** The container can't run the
  host's programs. In a toolbox, `dev-session.sh` sets `MERIDIAN_HOST_APPS=1`,
  and the shell then reads the host's desktop entries (`/run/host/usr/share`,
  Flatpak exports, `~/.local/share/applications`) and launches them **on the
  host** with `flatpak-spawn --host gio launch <file>`. They open inside the
  nested session.
- **XWayland in the toolbox.** The script gives nested labwc a private
  `/tmp/.X11-unix` in its own mount namespace, because the host's shows up
  as owned by `nobody` in a rootless container and labwc won't start
  XWayland there.
- **Options controls change your real hardware** (Wi-Fi, Bluetooth,
  backlight, volume), even in the nested session.
- **Sleep and Sign out act on your real session, not the nested one.**
  They go through logind for the process that's actually running
  `meridian-shell` — your host (or toolbox) login session — so clicking
  them suspends your real machine or ends the session you're developing
  from. Lock is safe to try: it only locks screens it can reach
  (`gtk4_session_lock::is_supported()` must be true for the nested labwc,
  which it is).

### Running on the host instead

To skip the container entirely, install the runtime pieces once:

```sh
sudo dnf install webkitgtk6.0 gtk4-layer-shell labwc wireplumber
```

Then run the toolbox-built binary on the host. The app list then comes from
GIO directly, with no bridge:

```sh
export XDG_CURRENT_DESKTOP=Meridian:wlroots MERIDIAN_UI_DIR=$PWD/ui/dist MERIDIAN_ASSETS_DIR=$PWD/assets
labwc -C session/labwc -S "$PWD/target/debug/meridian-shell"
```

## The login screen

```sh
tools/dev-greeter.sh
```

This runs `meridian-greeter` in a nested window against `tools/fake-greetd.py`,
which speaks greetd's protocol, accepts any username with the password
`meridian`, and only *prints* the session it would start.

## Install on this machine

```sh
tools/build-release.sh
sudo tools/install.sh            # binaries → /usr/local/bin, data → /usr/local/share/meridian,
                                 # session → /usr/share/wayland-sessions/meridian.desktop
```

`install.sh` also installs the runtime packages (WebKitGTK, gtk4-layer-shell,
labwc, Xwayland, WirePlumber, Inter, greetd). The login-screen switch is
separate: `--try-login-screen` (VT 7, alongside the current one),
`--enable-login-screen`, and `--disable-login-screen`. See
[ADR-0006](adr/0006-login-screen.md).

## Check

```sh
tools/check.sh
```

This runs rustfmt, clippy (`-D warnings`), all Rust tests, a check that the
generated TypeScript protocol types are committed, and the UI type-check and
build.

## Changing the protocol

1. Edit the types in `protocol/src/lib.rs`.
2. `cargo test -p meridian-protocol` regenerates `ui/src/lib/generated/*.ts`.
3. Update the result map in `ui/src/lib/bridge.ts` if you added a request,
   and the dispatcher in `shell/src/shell.rs`.
4. Give the request a capability in `Request::required_capability` and grant
   it only to the surfaces that need it.

ts-rs prints "failed to parse serde attribute: deny_unknown_fields" during
builds. That's expected: ts-rs doesn't model the attribute, and serde still
enforces it (see the `rejects_unknown_fields` test).

## Layout

See [ARCHITECTURE.md §13](ARCHITECTURE.md#13-repository-layout).
