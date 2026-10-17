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
