# First-use GUI plan

Design direction: a calm Windows-style desktop, plain white/near-black launcher,
clear blue selection states, consistent line icons, and readable light/dark modes.
Keep all existing file semantics and Debian setup work.

## This pass

- [x] Shared, theme-aware control and dialog styles; consistent SVG UI icons.
- [x] Taskbar: clear names/tooltips, balanced spacing, reliable pin reordering.
- [x] Desktop: optional dismissible first-use guidance, keyboard-friendly menus.
- [x] Launcher: keep the minimal search-first layout; improve keyboard/context actions.
- [x] Finder: functional keyboard navigation, useful empty states, persistent view,
      breadcrumbs/location entry, readable selection and folder artwork.
- [x] Settings: a useful Home page, real appearance previews, remove misleading
      controls, make available settings easy to find, retain honest feature limits.
- [x] Terminal: readable complete light/dark palettes and tab shortcuts.
- [x] Focus handling: menus and modal dialogs support Escape, Tab and arrow keys.
- [x] Verify: UI build/tests, native tests for any native changes, visual checks in
      light/dark and small windows. Keep a preview artifact and document limitations.

## Completion notes

Implemented a Settings Home page, appearance previews, working Sound controls,
consistent SVG icons, readable terminal themes, keyboard navigation, menu/dialog
focus handling, password visibility/Caps Lock feedback, optional desktop welcome,
clear desktop drop feedback, right-side native window controls, and Super+E/I/Enter
shortcuts. Sharing remains explicitly labeled as a preview; fake accent choices
and generic “Coming soon” rows were removed from the main experience.

Verification:

- UI TypeScript build and all 41 interaction tests passed, including terminal
  contrast, state-race handling, pinning, keyboard navigation and focus traps.
- All 115 Rust workspace tests passed; shell and greeter binaries built.
- Browser visual audit at 1280, 720 and 420 pixels, in both appearances: no
  unhandled page errors or unintended horizontal overflow.
- Started the real shell in a headless labwc compositor and opened Settings,
  Finder and Applications through exported actions. Captured native window
  controls and WebKitGTK surfaces in `docs/previews`.
- Shell scripts pass syntax checks and the Rust code passes formatting checks.

Browser screenshots use a deterministic read-only preview bridge. Native
screenshots used isolated XDG configuration and a private session bus. Hardware
availability, display-manager installation, and live cross-application drag/drop
still need interactive validation on the target desktop. Headless software
rendering emitted Vulkan/portal warnings; captures completed successfully.

A Chromium mouse-driven cross-frame drag attempt did not complete in the preview
harness; the drag/pin requests are covered by the interaction tests, but this is
not recorded as an end-to-end compositor drag/drop pass.

## Next GUI and feature pass

Priority order, starting with everyday window management:

- [x] Set GTK 3/4 and GNOME titlebar preferences to show minimize, maximize,
      and close on the right; preserve other GTK settings.
- [x] Explicitly request labwc server decorations and maximized titlebars.
- [x] Add Super+Down (minimize), Super+Up (maximize/restore), Super+Left/Right
      (snap), and Alt+Space (window menu); list them in Settings Home.
- [x] Match Settings searches against common terms such as audio, wifi,
      scaling, dock, dark, and keyboard.
- [ ] Validate decorations with GTK, libadwaita, Qt, Electron, and XWayland
      apps in the installed session, including maximized and fullscreen states.
- [ ] Add taskbar window previews and a picker for multiple windows of one app.
- [ ] Add a taskbar calendar with month navigation and keyboard support.
- [ ] Add audio output selection and mute, backed by WirePlumber.
- [ ] Add startup application management with explicit enable/disable controls.
- [ ] Add notification history and do-not-disturb using a native notification service.
- [ ] Audit keyboard focus, contrast, reduced motion, and fractional scaling across
      every surface on the target desktop.

Decoration preferences take effect when the shell next starts; some clients must
be reopened. Apps with custom titlebars may ignore desktop button preferences.
Do not force a blanket server-decoration window rule: it would duplicate the
header on clients that already draw one. Fullscreen intentionally hides controls.

## System management pass

- [x] Driver Manager: searchable module inventory, built-in/in-use status,
      confirmed authenticated session enable/disable.
- [x] Users: create/delete, administrator roles, passwords, login locking and
      current/last-administrator protections.
- [x] General: hardware/OS details and manual GitHub release version checking.
- [x] Terminal: normalized wheel/touchpad scrollback, history shortcuts, visible
      scrollbar, jump-to-latest action and avoid redundant emulator resizing.
- [x] Include PolicyKit agent and service dependencies in installed sessions.
- [ ] Verify real driver/account operations and terminal scrolling in the target
      installed session. See `SETTINGS-MANAGEMENT.md` for checks and limitations.
