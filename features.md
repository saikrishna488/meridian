# Meridian — Feature Roadmap & Acceptance Checklist

> **Goal:** Build a fast, coherent, premium desktop environment. Priorities are intended implementation order, not claims about what Meridian currently supports. Validate platform capabilities and repository state before implementing each feature.

## P0 — Design-system foundation (must happen first)

- [ ] Create one globally consumed token/theme package; eliminate conflicting local themes.
- [ ] Enforce dark-only UI everywhere; remove light mode switches, assets, branches, and obsolete preferences safely.
- [ ] Render primary backgrounds in pure `#000000` across owned desktop surfaces.
- [ ] Enforce `#FFFFFF`/`#000000` as the only text RGB colors (including hover, focus, selected, disabled); white/black opacity can vary.
- [ ] Implement one centrally stored, persisted hover-background preference: `Auto`, `Green`, `Blue`, `Grey`.
- [ ] Ensure hover text/glyph always stays white/black and remains readable with selected fill.
- [ ] Publish/reuse shared `Toggle`, `IconButton`, `Button`, `Slider`, `Popover`, `Menu`, `Tooltip`, `Dialog`, `IconTile`, `QuickSettingTile`, and other repeated components.
- [ ] Establish uniform small glyph sizes, icon-tile geometry, spacing, radii, borders, and motion tokens.
- [ ] Adopt macOS-inspired **solid** rounded icon backgrounds from blue/white/gray/green; stable assignment and white/black glyphs.
- [ ] Add component tests and a visual review route/story gallery if stack supports it.

## P0 — Visible desktop redesign

- [ ] **Control Panel:** compact icon-only toggle grid; **zero visible control labels** in default grid.
- [ ] Control Panel: shared toggle visuals/states; compact functional sliders or expandable secondary controls.
- [ ] Control Panel: tooltips, accessible names, full keyboard interaction, clear unavailable states.
- [ ] Launcher: compact search, small uniform icons, strong keyboard-first interaction.
- [ ] Dock/taskbar: compact uniform icons, shared hover behavior, clear active/running indicators.
- [ ] Desktop shell and all owned menus/popovers: dark-only, black backgrounds, unified borders, spacing, and focus behavior.
- [ ] Settings: remove light mode completely; add persisted global hover-background selector and preview.

## P1 — Core desktop capabilities

Implement or improve **only based on supported OS integration and actual project architecture**:

- [ ] Reliable app launching, tracking, closing, and window/task switching.
- [ ] Window move/resize/maximize/minimize behavior and workspace navigation, if Meridian is responsible for them.
- [ ] Robust input/focus handling and keyboard shortcut discovery.
- [ ] Notifications and Do Not Disturb with working system integration.
- [ ] Accurate Wi-Fi, Bluetooth, volume, microphone, brightness, display, battery, and power state where supported.
- [ ] Persistent preferences and resilient state restoration.
- [ ] Fast, informative search with no laggy launch feedback.
- [ ] Efficient rendering and responsive animations with reduced-motion support.
- [ ] Graceful offline, disconnected, permission-denied, and unsupported-device behavior.

## P1 — Polish & quality

- [ ] Consistent hover, pressed, selected, disabled, loading, error, and keyboard-focus states in every app.
- [ ] No hard-coded alternative text colors or unexplained gray surface backgrounds.
- [ ] No light flashes, bright default native popups, or theme mismatches where Meridian can control styling.
- [ ] Clear action affordances despite fewer visible labels.
- [ ] Pixel-consistent common icons, toggles, and menus at different resolutions/scaling factors.
- [ ] Useful empty states, progress indicators, and errors with minimal noise.
- [ ] Avoid unnecessary glow, gradients, blur, visual clutter, and oversized controls.

## P2 — Further world-class improvements

- [ ] Unified app/command search and contextual quick actions.
- [ ] Better window/workspace organization, where feasible in the OS architecture.
- [ ] System-wide command palette and discoverable shortcuts.
- [ ] Efficient multi-monitor behavior and scale consistency.
- [ ] Cohesive first-party apps/utilities and fine-grained settings search.
- [ ] Comprehensive accessibility review and keyboard-only navigation audit.
- [ ] Profiling-driven startup/memory/GPU and responsiveness improvements.

## Implementation sequence for autonomous coding agents

**Phase 1 — Inspect:** Map present desktop surfaces, code ownership, styling architecture, state management, available OS integrations, and test commands. Note existing behavior that must not regress.

**Phase 2 — Centralize:** Introduce tokens, shared primitives, shared icon mapping, global hover mode, and persistence. Define UI contracts before per-surface redesign.

**Phase 3 — Redesign P0:** Apply foundation to Control Panel first; then Settings, launcher, dock, desktop chrome, menus, and popovers. Remove duplicated components as each screen migrates.

**Phase 4 — Functional validation:** Wire all visuals to real state, validate keyboard/accessibility, and run unit/integration tests plus representative visual checks.

**Phase 5 — Audit:** Search for forbidden light-mode paths, nonblack main backgrounds, non-white/black text colors, oversized/inconsistent icons, unshared toggles, stray labels in quick controls, and hard-coded hover styles.

## Completion criteria / test matrix

| Test | Expected result |
|---|---|
| Appearance | Light mode is absent; every Meridian-owned shell surface is dark-only. |
| Background | Main desktop/panel/window backdrops are pure black. |
| Typography | Text glyph colors are white or black only, including hover. |
| Hover | Auto, Green, Blue, Grey settings are persisted and consistent desktop-wide. |
| Contrast | White/black text and icon glyphs stay legible on colored hover and icon tile backgrounds. |
| Components | Shared toggles, buttons, menus, sliders, tooltips, and icons are used across surfaces. |
| Control Panel | Small, uniform icon-only toggles; no visible labels in main toggle grid. |
| Accessibility | Icon-only controls expose names, tooltips, keyboard operation, and visible focus. |
| Functional state | Controls reflect true backend state, including failure/unavailable conditions. |
| Efficiency | Animations feel responsive; no unnecessary background work or heavy visual effects. |
| Regression | Existing working app/session/window/input behavior remains intact. |

## Agent reporting template

For every meaningful change, report:

1. **What changed:** modules/screens and shared components touched.
2. **How it follows the design:** tokens, text/background rules, hover mode, icon sizing, and component reuse.
3. **Validation:** commands/tests run and manual scenarios checked.
4. **Known gaps:** incomplete platform integration or unverified behavior.
5. **Next highest-priority task:** one concrete step from this roadmap.

Do not mark checklist items complete without implementation and verification.
