# Meridian

Meridian is a Linux desktop session for modern Debian, Fedora, and Arch Linux systems. It combines the labwc Wayland compositor with a Rust/GTK shell and
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
  local users, storage, power and screen locking, search providers, wallpaper,
  and system information. Wi-Fi and Bluetooth discovery starts only when you
  choose **New connection** or **Pair new device**; current and saved
  connections load when Settings opens. The Bluetooth page describes PipeWire
  codec quality, latency and range tradeoffs, choose Auto or Highest quality,
  and view the active codec when reported by PipeWire. Meridian's interface
  uses the light Precision style with rounded cards and green active controls.
  Settings includes recovery controls, accessibility preferences and an optional
  first-run checklist. Settings has eight main categories; Accessibility groups display and input
  preferences, while System contains maintenance and advanced options. Search
  still reaches individual settings pages.
- Control Center for connectivity, volume, brightness, Do Not Disturb, lock,
  sleep, restart, shutdown, and sign out. Meridian provides its own compositor-
  enforced lock screen on every display.
- Meridian's PipeWire idle-inhibit helper keeps the session awake during active
  audio playback. It is built from the pinned upstream source by
  `tools/compile_meridian.sh` and installed by `tools/install.sh`.
- Applications management with reviewed uninstall plans for supported Debian,
  Flatpak, and Snap packages. Privileged removals can request authentication.
- Actionable notification toasts, history, per-app alert controls and timed Do Not Disturb. Pulse system monitoring. Meridian's Calendar and
  Clock entries launch the installed GNOME Calendar and GNOME Clocks apps.
- Meridian Files for file browsing and Meridian Terminal for interactive shells.
  Other installed applications remain independently available.

System sharing is not implemented. Hardware controls depend on services such
as NetworkManager, BlueZ, PipeWire/WirePlumber, and systemd-logind, plus the
drivers available on the computer. See the
[architecture and implementation limits](docs/ARCHITECTURE.md) and
[Settings management notes](docs/SETTINGS-MANAGEMENT.md).

## Everyday desktop workflows

- **Settings → System → Recovery** reloads individual desktop components and shows recent session errors.
- **Settings → Accessibility** provides text scaling, high-contrast controls and reduced motion. Keyboard contains editable Meridian shortcuts with individual reset.
- **Control Center → Sound** selects a playback output and exposes application stream volume/mute controls. Refresh after audio hotplug or starting playback.
- Confirmed monitor layouts are remembered per monitor combination. The **Alt+Space** window menu includes relocation and recovery actions.
- Screenshots open a native crop/annotation preview. The original is saved to Pictures/Screenshots; copy or save a PNG copy to keep edits.
- Desktop layout and dock pin changes have undo buttons and Ctrl/Super+Z when focused. Closed read-only surfaces are reclaimed after five idle minutes; active work is retained.

These source additions require a rebuilt installation and a new session. Their
live hardware, accessibility and performance qualification is still pending.
See [workflow scope and manual review](docs/DESKTOP-WORKFLOWS.md).

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
Debian testing remains the CI baseline; Fedora, Arch, and derivatives need matching
versions. The development setup script installs the required host packages.

## Try it in a development session

On a supported distribution with the required GTK, WebKitGTK, labwc, Rust, and Node
versions:

```sh
tools/dev-env.sh
tools/dev-session.sh
```

This runs Meridian in a nested compositor window. To try the login screen with
a fake greeter, use `tools/dev-greeter.sh`; its test password is `meridian`.
For development checks, run `tools/check.sh`.

## Install on Linux

Install from one source checkout command (run as your regular user, **without** sudo):

```sh
bash install.sh
```

The default command opens the native graphical installer. `--gui` remains an
explicit alias; use `--cli` for terminal operation.

After installation, open **Meridian Installer** from Applications or
**Settings → General → Open Installer**. Choose a source checkout and install,
update, repair, compile only, check requirements, or remove Meridian. **Use prepared release**
skips compilation when the release matches the checkout; source rebuilds take
longer. First GUI launch may compile the GTK-only installer and requires the
Rust/GTK development prerequisites. The GUI itself uses no Node or WebKit.

The shared installer backend detects APT, DNF, or pacman from `/etc/os-release`, shows a
confirmation before system changes, installs build/runtime dependencies, compiles
a verified release, and invokes the transactional privileged installer. It
keeps the current login manager unchanged. Terminal commands through the same entry point:

```sh
bash install.sh --cli               # compile and install through the terminal
bash install.sh --cli --compile     # compile a verified release without installing Meridian
bash install.sh --cli --check       # read-only dependency and version report
bash install.sh --cli --update      # rebuild and reinstall current checkout
bash install.sh --cli --repair      # rebuild and reinstall current checkout
bash install.sh --cli --uninstall   # remove Meridian-managed system files
```

`--cli --update` intentionally does not pull Git changes; run `git pull` on your
desired branch first. `--yes` skips the package-installation confirmation for
reviewed automation. Compilation and installation use this same entry point;
`--compile` builds only, and `--prepared` installs an already-verified build.
The lower-level helper scripts remain available for compatibility.

Setup selects APT, DNF, or pacman from `/etc/os-release`. Arch setup performs
a full system upgrade. Fedora codecs come from its configured repositories and
may support fewer formats. New profiles have script tests; desktop and hardware
qualification on Fedora and Arch remains pending. Systemd/logind is required.

Select **Meridian** at the login screen. To update, pull the source you want and run `bash install.sh --cli --update`, then
sign out and back in. Remove it with `bash install.sh --cli --uninstall`. See [installation details](tools/INSTALLER.md)
for optional greetd configuration.

## Rendering engines

Open **Engines** in Applications or **Settings → Engines** for four automatically
saved switches: Font, GPU, CPU and Audio. Font/GPU/native CPU changes need a new
session. Audio configures high-quality PipeWire resampling for new streams;
playback apps and the PulseAudio service need restarting to pick up changes.
The four precompiled C++ libraries total about 21 KB, excluding existing system
libraries. See [engines](docs/RENDERING-ENGINES.md) for scope and limitations.

## Keyboard shortcuts

| Keys | Action |
| --- | --- |
| Super or Super+Space | Open Applications |
| Super+C | Open Control Center |
| Super+E | Open Meridian Files |
| Super+I | Open Settings |
| Super+Enter | Open Meridian Terminal |
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
- [Third-party components](docs/THIRD-PARTY.md)
- [Phase 1 stability qualification](docs/PHASE1.md)
- [Phase 2 performance qualification](docs/PHASE2.md)
- [Phase 3 workflow work and remaining gaps](docs/PHASE3.md)
- [Historical UI captures](docs/previews/README.md)

## License

Meridian is free software under the [GNU GPL, version 3 or later](LICENSE).
