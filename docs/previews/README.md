# Historical UI captures

**These screenshots are from earlier Meridian revisions and are not current
product screenshots.** The current UI is dark-only, the production launcher
opens GNOME Files and Kitty instead of the old embedded Orbit and terminal
surfaces, and the current desktop chrome has changed since these captures.
Do not use the images below to judge the appearance or application list in the
current package.

Current-build screenshots have not yet been captured. The latest design pass
records why visual captures could not be produced in its execution environment
in [UI design validation](../UI-DESIGN-VALIDATION.md). Once a desktop capture
environment is available, use the read-only preview bridge for app surfaces and
a native labwc session for the composed desktop. Follow the capture matrix in
that validation note so sample data and capture conditions are clear.

## Earlier native desktop captures

These show the previous desktop layout and, in some cases, the previous light
appearance:

- [Desktop and taskbar](modern-native-desktop.png)
- [Settings with native window controls](modern-native-settings.png)
- [Earlier Settings window](native-settings.png)
- [Earlier Finder window](native-finder.png)
- [Earlier launcher above the taskbar](native-launcher.png)

## Earlier browser and component previews

These deterministic previews use sample data. Their light/dark variants and
old Orbit/Finder app design are retained as history, not as current feature
claims:

- Settings Home: [earlier light](modern-settings-light.png) · [earlier dark](modern-settings-dark.png)
- Applications launcher: [earlier light](modern-launcher-light.png) · [earlier dark](modern-launcher-dark.png)
- Old Orbit file manager: [earlier light](modern-orbit-light.png) · [earlier dark](modern-orbit-dark.png)
- Startup Apps: [earlier preview](modern-startup-light.png)
- Sound: [earlier preview](modern-sound-light.png)
- Power and lock: [earlier preview](modern-power-dark.png)
- Trash: [earlier preview](modern-trash-light.png)
- Notifications: [earlier preview](modern-notifications-light.png)
- Control Center: [earlier light](refined-controls-light.png) · [earlier dark](refined-controls-dark.png)
- Launcher refinements: [earlier light](refined-launcher-light.png) · [earlier dark](refined-launcher-dark.png) · [list view](refined-launcher-list.png)
- Lock screen: [earlier preview](refined-lock-screen.png)

The preview bridge changes only in-memory fixtures. It does not exercise real
hardware, system services, package management, or file mutations. For current
project status and implementation details, see the [README](../../README.md),
[architecture](../ARCHITECTURE.md), and [qualification notes](../PHASE3.md).
