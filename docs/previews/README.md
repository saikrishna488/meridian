# GUI previews

Native Wayland captures from a private headless labwc session (isolated XDG
configuration and session bus):

- [Settings with native window controls](native-settings.png)
- [Finder with native window controls](native-finder.png)
- [Launcher above the taskbar](native-launcher.png)

Read-only browser previews with deterministic sample data:

- [Light desktop](desktop-light.png) · [Dark desktop](desktop-dark.png)
- [Light Settings Home](settings-light.png) · [Dark Settings Home](settings-dark.png)

Screenshots are review artifacts, not promises of hardware/service availability.
The preview bridge only changes its in-memory fixtures. Production still uses
the native capability-checked bridge. To open the local preview, build `ui/`
and run `node tools/preview-ui.mjs` from the repository root.

October desktop refinement, captured in WebKit inside an isolated headless labwc
session with the deterministic preview bridge:

- [Settings Home, light](refined-settings-light.png) · [dark](refined-settings-dark.png)
- [Applications, light](refined-launcher-light.png) · [dark](refined-launcher-dark.png)
- [Control Center and power actions, light](refined-controls-light.png) · [dark](refined-controls-dark.png)

[Design changes and screen power behavior](../DESKTOP-DESIGN.md).
- [Compact application list](refined-launcher-list.png)
- [Lock screen](refined-lock-screen.png)
