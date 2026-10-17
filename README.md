# Meridian

A Linux desktop shell with a Windows-style bottom taskbar, searchable application
launcher, Control Center, desktop shortcuts, and built-in Finder, Terminal,
and Settings windows.

Meridian runs on labwc/Wayland and existing Linux services. Its native host is
written in Rust; React, TypeScript, HTML, and CSS provide the interface inside
WebKitGTK. GTK4 hosts the web surfaces and native window controls.

**Status:** working prototype. Wallpaper, appearance, networking, Bluetooth,
display configuration, file operations, and terminal sessions are implemented.
Some settings remain placeholders, and Sharing is a UI-only prototype.
See [the architecture](docs/ARCHITECTURE.md) for implementation details and limits.

## Debian requirements

The current checkout needs Rust >= 1.92, GTK >= 4.18, WebKitGTK >= 2.52, gtk4-layer-shell >= 1.2, and Node >= 22.22.2. Development and CI target Debian testing with matching packages; Debian derivatives with those versions also work. Older stable releases need compatible backports; the scripts do not change your APT sources. Development dependencies are installed directly on the host.

## Try it

```sh
tools/dev-env.sh        # one-time: install Debian development packages on the host
tools/dev-session.sh    # build and run Meridian in a nested window
tools/dev-greeter.sh    # try the login screen in a window (fake login, password "meridian")
tools/check.sh          # fmt, clippy, tests, UI type-check
```

## Install (Debian)

```sh
tools/build-release.sh                       # as your user
sudo tools/install.sh                        # adds "Meridian" to your login screen's session list
sudo tools/install.sh --try-login-screen     # optional: Meridian's login screen on Ctrl+Alt+F7
sudo tools/install.sh --enable-login-screen  # optional: make it your login screen (after reboot)
sudo tools/install.sh --disable-login-screen # undo the above
sudo tools/install.sh --uninstall
```

| Keys | Action |
|---|---|
| Super / Super+Space | Applications menu (type to search, ↑ ↓, Enter, Esc) |
| Super+C | Control Center |
| Super+E | Finder |
| Super+I | Settings |
| Super+Enter | Terminal |
| Super+L | Lock |
| Window menu → window | focus that window |
| Super+Q | close the focused window |
| Alt+Tab | switch windows |
| Super+Shift+Q / Ctrl+Alt+Backspace | log out |

In the nested dev session the host desktop usually grabs Super, so use
**Ctrl+Alt+H** (Applications), **Ctrl+Alt+C** (Options), and **Ctrl+Alt+L**
(Lock), or use the dock and Control Center.

Documentation: [architecture](docs/ARCHITECTURE.md) ·
[development](docs/DEVELOPMENT.md) · [decision records](docs/adr/)

## License

Meridian is free software under the [GNU GPL, version 3 or later](LICENSE).

## Dock, Settings, and desktop icons

Drag dock items (including the Applications launcher and Trash) to reorder them. The layout is saved in `~/.config/meridian/dock-layout.json`. The taskbar stays visible while Applications is open and hides for fullscreen applications. New configurations pin Meridian Terminal and Finder alongside the Applications launcher; existing saved pins are preserved.

Click Trash to open the file manager's trash. Drag an installed app onto Trash, or right-click an app and choose **Uninstall…**, to review and confirm removal. Meridian resolves the desktop entry to an exact Flatpak app ref or its Debian package. Flatpak removal keeps saved app data. Debian package removal uses `pkexec apt-get`, may require authentication, and can remove dependent packages; this is described in the confirmation. Unmanaged launchers show an unsupported-package error rather than deleting a shortcut. Uninstalling multiple apps at once is prevented.

Open **Meridian → Settings…**, or find **Settings** in Applications. Wallpaper,
light/dark appearance, Wi-Fi, Bluetooth, and display settings are available.
Software Update, Startup Applications, Sound preferences, and Privacy & Security
are not complete; volume can be adjusted in Control Center. Sharing is a UI
prototype and does not enable sharing services.

Installed apps use their desktop-entry artwork in consistent rounded tiles.
Finder, Settings, Terminal, the launcher, Control Center, and Trash use fixed
Meridian SVG artwork. See [desktop icons](assets/desktop-icons/README.md).

**Settings → Wallpaper** includes Dusk, Tide, Alpine, and Aurora, original GPL-licensed SVG wallpapers. A native picker accepts custom PNG/JPEG/WebP images up to 20 MB and saves a copy, so moving the original image does not break the wallpaper. Your choice and desktop shortcuts are saved in `~/.config/meridian/desktop.json`.

The desktop shows files and folders in your XDG Desktop folder (normally `~/Desktop`) and updates when that folder changes. Dragging files from Finder or another file manager onto the desktop moves them into that folder; existing names are never overwritten. Finder also offers **Move to Desktop**. **Move to Trash** sends a real desktop file to the system Trash. Old saved shortcuts remain shortcuts and can be removed without touching their original files.

Drag apps from Applications onto the taskbar to pin them, or right-click an app and select **Add to Dock**. Apps can also be added to the desktop as application shortcuts. Right-click the desktop to sort items, move files here, open Finder/Applications, or change wallpaper.

Sleep, Restart, Shut Down, Lock, and Sign out are in the **Meridian** menu. Restart, Shut Down, and Sign out require a second click to confirm. Control Center keeps connectivity, appearance, brightness, and volume controls.

## Finder and Terminal

Open **Meridian → Finder… / Terminal…**, or search Applications for **Finder** or **Meridian Terminal**. Both have fixed Meridian window controls and can be pinned to the dock. Built-in apps cannot be uninstalled. The bundled labwc theme also supplies original controls for compositor-decorated windows; third-party apps that draw their own titlebars control their own decorations.

Finder browses actual folders, with back/forward navigation, an enclosing-folder button, icon/list views, filtering within the current folder, hidden files, refresh, and a path field. Use **New folder** or a file’s context menu to open, rename, add to the desktop, or move to the system Trash. Rename never overwrites another item. **Applications** lists installed apps with Open, Add to Dock, Add to Desktop, and reviewed Uninstall actions.

Terminal runs your actual interactive shell on a PTY using the locally bundled, MIT-licensed xterm.js renderer. It supports tabs, ANSI colors, shell history and completion, interactive programs, resizing, Ctrl+C, Ctrl+Shift+C to copy a selection, and Ctrl+Shift+V to paste. Closing a tab/window ends its shell and foreground job; detached jobs may survive. Development and installed sessions run on the Debian host.

## Multimedia codecs

The Debian installer installs GStreamer base/good/bad/ugly plugins, the FFmpeg bridge, and FFmpeg from your configured APT repositories. No third-party codec repository is enabled. Flatpak players carry their own runtime codecs.

Run `sudo tools/install-codecs.sh` to install codecs separately. The legacy `--full` and `--full-codecs` options use the same Debian package set.

## Appearance

Appearance can be changed in Settings → Appearance. The saved Light/Dark preference updates Meridian, GTK settings, the desktop color-scheme preference and the KDE palette when its color-scheme tool is installed. Apps with their own theme override can keep that override. App-menu icons and Finder files or folders can be dragged onto the desktop to create shortcuts; external file-manager drops are accepted through GTK.

Every Meridian window reads the saved appearance when it opens and listens for live changes through the shared UI appearance hook (`ui/src/lib/appearance.ts`). The hook updates the WebView color scheme and document theme, so native controls and app styles can follow the same setting. Settings and Terminal use both light and dark palettes; the terminal emulator uses complete, high-contrast light and dark palettes and repaints existing output after appearance changes. Applications outside Meridian may keep their own appearance setting.

## Displays and connectivity

Settings includes multiple displays (resolution, refresh rate, scaling, rotation and position), hardware brightness, Wi-Fi discovery and joining, and Bluetooth discovery, pairing and connections. Monitor changes are tested by the compositor and revert after 15 seconds unless kept; kept layouts are saved for the next session. Wi-Fi passwords go to nmcli over stdin and are redacted from bridge logs. Bluetooth pairing uses a temporary Meridian agent with PIN/passkey confirmation prompts.

## Menu bar and Control Center

The bottom taskbar places the Meridian menu on the left, pinned and running apps in the middle, and Control Center and the clock on the right. Menus open above the taskbar.

Control Center groups connectivity, appearance, display and sound into rounded cards. Sign out is in the Meridian menu and requires confirmation. UI switches share the accessible Toggle component and UI type uses the free Inter family; GTK and Qt desktop defaults also use Inter, while terminal cells use a monospace face. Sharing is an interactive UI prototype and does not start sharing services. External monitor brightness uses the monitor’s hardware controls.

## GUI development and previews

Settings opens to a Home page with useful preferences and keyboard shortcuts. An empty desktop offers a dismissible welcome card; dismissal is remembered. Finder supports arrow keys, F2, F5, Ctrl+F, Ctrl+L and Alt+Left/Right. Terminal supports Ctrl+Shift+T/W and Ctrl+PageUp/PageDown for sessions.

For a read-only UI preview without starting a desktop session:

```sh
npm run build --prefix ui
node tools/preview-ui.mjs
```

Open `http://127.0.0.1:4173`. The preview uses in-memory sample data and never runs native file, login or hardware operations. Replace `settings` in `/ui/surfaces/settings/index.html` with `finder`, `applications`, `options`, `terminal`, `wallpaper`, `greeter` or `locker`; add `?theme=dark` for dark mode. [GUI plan and verification](docs/GUI-POLISH.md) · [Visual previews](docs/previews/README.md).
