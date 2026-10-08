# Meridian

A Linux desktop shell with a menu bar, icon-based dock, searchable application
launcher, Control Center, desktop shortcuts, and built-in Finder, Terminal,
and Settings windows.

Meridian runs on labwc/Wayland and existing Linux services. Its native host is
written in Rust; React, TypeScript, HTML, and CSS provide the interface inside
WebKitGTK. GTK4 hosts the web surfaces and native window controls.

**Status:** working prototype. Wallpaper, appearance, networking, Bluetooth,
display configuration, file operations, and terminal sessions are implemented.
Some settings remain placeholders, and Sharing is a UI-only prototype.
See [the architecture](docs/ARCHITECTURE.md) for implementation details and limits.

## Try it

```sh
tools/dev-env.sh        # one-time: create the `meridian-dev` toolbox (nothing installed on the host)
tools/dev-session.sh    # build and run Meridian in a nested window
tools/dev-greeter.sh    # try the login screen in a window (fake login, password "meridian")
tools/check.sh          # fmt, clippy, tests, UI type-check
```

## Install (Fedora)

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
| Super+C | Options menu |
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

Drag dock items (including the Applications launcher and Trash) to reorder them. The layout is saved in `~/.config/meridian/dock-layout.json`. Opening Applications shows its dock and hides the desktop dock; fullscreen applications also hide the desktop dock.

Click Trash to open the file manager's trash. Drag an installed app onto Trash, or right-click an app and choose **Uninstall…**, to review and confirm removal. Meridian resolves the desktop entry to an exact Flatpak app ref or its RPM package. Flatpak removal keeps saved app data. RPM removal uses `pkexec dnf`, may require authentication, and can remove dependent packages; this is described in the confirmation. Unmanaged launchers show an unsupported-package error rather than deleting a shortcut. Uninstalling multiple apps at once is prevented.

Open **Meridian → Settings…**, or find **Settings** in Applications. Wallpaper,
light/dark appearance, Wi-Fi, Bluetooth, and display settings are available.
Software Update, Startup Applications, Sound preferences, and Privacy & Security
are not complete; volume can be adjusted in Control Center. Sharing is a UI
prototype and does not enable sharing services.

Installed apps use their desktop-entry artwork in consistent rounded tiles.
Finder, Settings, Terminal, the launcher, Control Center, and Trash use fixed
Meridian SVG artwork. See [desktop icons](assets/desktop-icons/README.md).

**Settings → Wallpaper** includes Dusk, Tide, Alpine, and Aurora, original GPL-licensed SVG wallpapers. A native picker accepts custom PNG/JPEG/WebP images up to 20 MB and saves a copy, so moving the original image does not break the wallpaper. Your choice and desktop shortcuts are saved in `~/.config/meridian/desktop.json`.

Right-click the desktop to add file shortcuts, open Applications/Finder, or change the wallpaper. Apps can be dragged from Applications/Finder onto the desktop, or added from their context menu. Finder can also add files and folders to the desktop. Double-click a shortcut to open it; **Remove shortcut** keeps its original file.

Sleep, Restart, Shut Down, Lock, and Sign out are in the **Meridian** menu. Restart, Shut Down, and Sign out require a second click to confirm. Control Center keeps connectivity, appearance, brightness, and volume controls.

## Finder and Terminal

Open **Meridian → Finder… / Terminal…**, or search Applications for **Finder** or **Meridian Terminal**. Both have fixed Meridian window controls and can be pinned to the dock. Built-in apps cannot be uninstalled. The bundled labwc theme also supplies original controls for compositor-decorated windows; third-party apps that draw their own titlebars control their own decorations.

Finder browses actual folders, with back/forward navigation, an enclosing-folder button, icon/list views, filtering within the current folder, hidden files, refresh, and a path field. Use **New folder** or a file’s context menu to open, rename, add to the desktop, or move to the system Trash. Rename never overwrites another item. **Applications** lists installed apps with Open, Add to Dock, Add to Desktop, and reviewed Uninstall actions.

Terminal runs your actual interactive shell on a PTY using the locally bundled, MIT-licensed xterm.js renderer. It supports tabs, ANSI colors, shell history and completion, interactive programs, resizing, Ctrl+C, Ctrl+Shift+C to copy a selection, and Ctrl+Shift+V to paste. Closing a tab/window ends its shell and foreground job; detached jobs may survive. Installed sessions run on the host; development sessions started in a toolbox run the shell inside that toolbox (your home directory is shared).

## Multimedia codecs

The Fedora installer includes GStreamer base/good/bad-free/ugly-free plugins, the FFmpeg bridge, FFmpeg-free, and Cisco OpenH264 for common media playback. Existing full FFmpeg installations are preserved. Codecs are installed by the installer, not by starting a development session; Flatpak players carry their own runtime codecs.

For broader codec support, including H.265, run `sudo tools/install.sh --full-codecs`, or `sudo tools/install-codecs.sh --full` to add only codecs. This enables **RPM Fusion Free** and adds its codec extensions, with a targeted ugly-plugin swap. It does not swap the entire FFmpeg stack or change GPU drivers. Default Fedora-only codecs can be installed separately with `sudo tools/install-codecs.sh`.

Package references: [Fedora OpenH264](https://fedoraproject.org/wiki/OpenH264), [Fedora video acceleration and codec options](https://fedoraproject.org/wiki/Hardware_Video_Acceleration), [RPM Fusion multimedia](https://rpmfusion.org/Howto/Multimedia).

## Appearance

Appearance can be changed in Settings → Appearance. The saved Light/Dark preference updates Meridian, GTK settings, the desktop color-scheme preference and the KDE palette when its color-scheme tool is installed. Apps with their own theme override can keep that override. App-menu icons and Finder files or folders can be dragged onto the desktop to create shortcuts; external file-manager drops are accepted through GTK.

Every Meridian window reads the saved appearance when it opens and listens for live changes through the shared UI appearance hook (`ui/src/lib/appearance.ts`). The hook updates the WebView color scheme and document theme, so native controls and app styles can follow the same setting. Settings and Terminal use both light and dark palettes; the terminal emulator itself stays black on white for readable shell output and repaints after appearance changes. Applications outside Meridian may keep their own appearance setting.

## Displays and connectivity

Settings includes multiple displays (resolution, refresh rate, scaling, rotation and position), hardware brightness, Wi-Fi discovery and joining, and Bluetooth discovery, pairing and connections. Monitor changes are tested by the compositor and revert after 15 seconds unless kept; kept layouts are saved for the next session. Wi-Fi passwords go to nmcli over stdin and are redacted from bridge logs. Bluetooth pairing uses a temporary Meridian agent with PIN/passkey confirmation prompts.

## Menu bar and Control Center

The menu bar shows the active application name, Meridian menus, Control Center,
and a clock. Its File/Edit/View/Window/Help menus are shell actions, not imported
application menus; app editing commands are currently disabled.

Control Center groups connectivity, appearance, display and sound into rounded cards. Sign out is in the Meridian menu and requires confirmation. UI switches share the accessible Toggle component and UI type uses the free Inter family; GTK and Qt desktop defaults also use Inter, while terminal cells use a monospace face. Sharing is an interactive UI prototype and does not start sharing services. External monitor brightness uses the monitor’s hardware controls.
