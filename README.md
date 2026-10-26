# Meridian

Meridian is a Linux desktop session for Debian testing and compatible
derivatives. It combines the labwc Wayland compositor with a Rust/GTK shell and
a React interface rendered by WebKitGTK.

**Project status:** installable prototype, not yet ready to be a user's sole
daily-use desktop. Stability, hardware, installation, accessibility, and
extended-use qualification remain open. The project tracks those gaps in
[Phase 1](docs/PHASE1.md), [Phase 2](docs/PHASE2.md), and
[Phase 3](docs/PHASE3.md).

## What works today

- A desktop with configurable wallpapers, desktop files and shortcuts, an
  application launcher, a reorderable dock, a top bar, window switching, and
  common window controls.
- Settings for Wi-Fi, Bluetooth, displays, sound, startup apps, installed apps,
  local users, devices, power and screen locking, search providers, wallpaper,
  and system information. Meridian's own interface is dark-only; Appearance
  lets you choose a system font.
- Control Center for connectivity, volume, brightness, Do Not Disturb, lock,
  sleep, restart, shutdown, and sign out.
- Applications management with reviewed uninstall plans for supported Debian,
  Flatpak, and Snap packages. Privileged removals can request authentication.
- Notification history and Pulse system monitoring. Meridian's Calendar and
  Clock entries launch the installed GNOME Calendar and GNOME Clocks apps.
- GNOME Files for file browsing and Kitty for interactive terminal sessions.
  They are separate applications launched from Meridian.

System sharing is not implemented. Hardware controls depend on services such
as NetworkManager, BlueZ, PipeWire/WirePlumber, and systemd-logind, plus the
drivers available on the computer. See the
[architecture and implementation limits](docs/ARCHITECTURE.md) and
[Settings management notes](docs/SETTINGS-MANAGEMENT.md).

## Screenshots

![Dusk, one of Meridian's bundled desktop wallpapers](assets/wallpapers/meridian-dusk.svg)

This is current bundled wallpaper artwork, not a desktop screenshot. The images
in [`docs/previews`](docs/previews/) are historical captures from earlier UI
revisions. Some show a light theme and old Orbit/Terminal app surfaces; they do
not represent the current package. Current-build screenshots still need to be
captured in a working desktop session.

## Build requirements

The current checkout needs Rust 1.92 or newer, GTK 4.18 or newer, WebKitGTK
2.52 or newer, gtk4-layer-shell 1.2 or newer, and Node.js 22.22.2 or newer.
Development and CI target Debian testing; Debian derivatives need matching
versions. The development setup script installs the required host packages.

## Try it in a development session

On Debian testing with the required GTK, WebKitGTK, labwc, Rust, and Node
versions:

```sh
tools/dev-env.sh
tools/dev-session.sh
```

This runs Meridian in a nested compositor window. To try the login screen with
a fake greeter, use `tools/dev-greeter.sh`; its test password is `meridian`.
For development checks, run `tools/check.sh`.

## Install on Debian

Build and install the Debian package:

```sh
tools/build-deb.sh --output /tmp/meridian.deb
sudo apt install /tmp/meridian.deb
```

Select **Meridian** at the login screen. After installing an update, sign out
and back in to load the updated session. Remove the package with
`sudo apt remove meridian-desktop`. See [installation details](tools/INSTALLER.md)
for the source installation method and optional greetd configuration.

## Keyboard shortcuts

| Keys | Action |
| --- | --- |
| Super or Super+Space | Open Applications |
| Super+C | Open Control Center |
| Super+E | Open GNOME Files |
| Super+I | Open Settings |
| Super+Enter | Open Kitty |
| Super+L | Lock the session |
| Print Screen | Capture the desktop |
| Super+Shift+S or Shift+Print Screen | Select a screenshot area |
| Alt+Tab | Switch windows |
| Super+Q | Close the focused window |
| Super+Down / Super+Up | Minimize / maximize or restore |
| Super+Left / Super+Right | Snap the window |
| Alt+Space | Open window actions |
| Super+Shift+Q or Ctrl+Alt+Backspace | Sign out |

In the nested development session, use Ctrl+Alt+H for Applications,
Ctrl+Alt+C for Control Center, and Ctrl+Alt+L to lock.

## Documentation

- [Architecture and current limits](docs/ARCHITECTURE.md)
- [Development guide](docs/DEVELOPMENT.md)
- [Settings management](docs/SETTINGS-MANAGEMENT.md)
- [Phase 1 stability qualification](docs/PHASE1.md)
- [Phase 2 performance qualification](docs/PHASE2.md)
- [Phase 3 workflow work and remaining gaps](docs/PHASE3.md)
- [Historical UI captures](docs/previews/README.md)

## License

Meridian is free software under the [GNU GPL, version 3 or later](LICENSE).
