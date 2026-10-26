# Developing Meridian

## Environment

Development targets Debian testing and compatible Debian derivatives. Run:

```sh
tools/dev-env.sh
```

This installs native APT packages on the host, using sudo when needed. It does not create a container or add repositories. The current Rust bindings require Rust >= 1.92, GTK >= 4.18, WebKitGTK >= 2.52, and gtk4-layer-shell >= 1.2. Node >= 22.22.2 is required by the UI test dependencies. Older Debian stable packages may be too old: use matching backports or a release providing these versions. The setup checks the native library versions after installation.

Build packages are listed in `tools/debian.sh`: Rust/Cargo/rustfmt/clippy, Node/npm, GTK4/WebKitGTK development libraries, gtk4-layer-shell, PAM, libclang, pkg-config and build-essential. Runtime packages include labwc, Xwayland, greetd, NetworkManager, BlueZ, WirePlumber, Inter and DDC utilities. PAM uses Debian’s common-auth/common-account stack.

Package references: [Debian gtk4-layer-shell](https://packages.debian.org/testing/libgtk4-layer-shell-dev), [Debian WebKitGTK](https://packages.debian.org/testing/libwebkitgtk-6.0-dev).

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
`subscribeAppearance()` or use the hook. Terminal emulator output follows the selected light/dark palette with a minimum contrast ratio for ANSI and truecolor text.

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
- **Options controls change your real hardware** (Wi-Fi, Bluetooth,
  backlight, volume), even in the nested session.
- **Sleep and Sign out act on your real session, not the nested one.**
  They go through logind for the process that's actually running
  `meridian-shell` — your host login session — so clicking
  them suspends your real machine or ends the session you're developing
  from. Lock is safe to try: it only locks screens it can reach
  (`gtk4_session_lock::is_supported()` must be true for the nested labwc,
  which it is).

## The login screen

```sh
tools/dev-greeter.sh
```

This runs `meridian-greeter` in a nested window against `tools/fake-greetd.py`,
which speaks greetd's protocol, accepts any username with the password
`meridian`, and only *prints* the session it would start.

## Install on this machine

```sh
tools/compile_meridian.sh
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
build. Run `cd ui && npm test` separately for UI interaction tests with a mocked
host; these do not validate real hardware.

## Changing the protocol

1. Edit the message types, validation, and capability definitions in `protocol/src/lib.rs`.
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

## Read-only GUI preview

`node tools/preview-ui.mjs` serves `ui/dist` on loopback port 4173 with sample fixtures. Build the UI first. It strips the native-only CSP and maps `meridian://` assets to local HTTP **only in the preview response**; production bundles and CSP remain unchanged. Fake bridge calls update in-memory sample state and never invoke native operations. See `docs/GUI-POLISH.md` for the visual and keyboard audit.
