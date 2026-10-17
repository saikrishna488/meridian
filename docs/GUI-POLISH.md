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
