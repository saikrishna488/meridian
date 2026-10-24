# Meridian — Apps & System Surfaces Specification

> This file defines how first-party apps and system surfaces should fit together. It is a design/implementation target, **not** a claim that all apps below already exist in the repository. Inspect current code and map names/capabilities before modifying or adding modules.

## Shared app contract

Every Meridian-owned app/surface must:

- Use the same global design tokens from `design.md`: **dark only**, pure black main backgrounds, white/black text only, and centralized hover colors.
- Reuse shared shell layout, header, icon buttons, toggles, menus, sliders, dialogs, notifications, empty/error states, and shortcuts.
- Keep navigation minimal and icon-first where the meaning is clear; add tooltips and accessible names to icon-only actions.
- Use uniformly sized small icons. When using solid icon tiles, restrict backgrounds to blue, white, gray, and green; glyphs stay white/black.
- Use real app/service/backend state rather than decorative placeholders. Handle permission failures, unavailable devices, pending state, and loading without lying to the user.
- Contribute to a shared desktop-wide keyboard, focus, search, and notification model.
- Follow consistent window behavior, state persistence, high-DPI scaling, and responsive compact layouts.

## Priority desktop surfaces

### 1. Desktop & workspace shell — P0

Black-first visual canvas with cohesive dock, status area, workspace boundaries, context menus, and window management. The shell should look unified in idle, multitasking, and full-screen states.

**Must:** stable window positioning/state, consistent corners, system menus, shortcuts, and coherent focus indicators. No light flashes during startup or transitions.

### 2. Control Panel / Quick Settings — P0

**Major redesign required.** Replace large labeled rows/cards with a **small, dense grid of icon-only toggle controls**. No visible labels in the default toggle grid. Use one shared `QuickSettingTile` / `Toggle` implementation with identical sizing, interaction, and state styling.

Potential controls, **only if supported by actual Meridian/OS integration**: Wi-Fi, Bluetooth, audio mute/output, microphone mute, brightness, night light, notifications/do-not-disturb, battery/power mode, keyboard settings, and display settings.

Use compact sliders for brightness/volume where functional. Show details/settings on secondary interaction, and tooltips/accessibility names for every icon. Disabled/unavailable hardware must be clearly differentiated. Hover fill uses global Auto/Green/Blue/Grey setting.

### 3. Launcher / universal search — P0

Fast keyboard-first app launch and search. Pure black background, minimal chrome, compact results, uniform icon sizing, clear selection, and predictable Enter/Escape behavior. Results can expose secondary information when useful, but should not become over-labeled tiles.

### 4. Dock / taskbar — P0

Small, consistent icons and predictable running/active indicators. Shared hover/focus/tooltip behaviors. Avoid oversized hover magnification or distracting effects. Integrate launcher and running-app behavior with real process/window state.

### 5. Settings — P0/P1

The central place for less-frequent configuration. More explanatory text is acceptable here than in the Control Panel, but typography stays only white/black. Group related sections compactly and reuse exactly the same toggle, slider, menu, search, and control components.

**Appearance section:** dark mode is fixed and not user-switchable. Include global `Hover Background` selector with `Auto`, `Green`, `Blue`, `Grey`, with preview and persisted selection. Do not include a light mode switch or any other text-color palette option.

### 6. Status bar & system popovers — P1

Compact network, audio, battery, time, notifications, and other status affordances. Shared iconography and menus. Popovers use pure-black surfaces, consistent borders, sizing, focus handling, and dismissal behavior.

### 7. Notifications / notification center — P1

Consistent compact cards or rows with black foundations and clear white/black text. Accessibility names for actions. Respect notification state, grouping, dismissal, quiet modes, and backend availability.

### 8. Window chrome & task switcher — P1

Unified titlebar and window actions where owned by Meridian. Clear hover and keyboard focus, small system icons, smooth efficient transitions, reliable focus and z-order, and accurate window previews if supported.

### 9. File manager & first-party utilities — P2 / as present

Any app included in Meridian must use the same shared components, icon system, hover preference, typography, and background rules. App-specific layouts are allowed; new independent visual systems are not.

### 10. Lock / login surface — P2 / only if Meridian owns it

Apply dark-only identity and consistent icons/buttons where technically possible. Coordinate with the system's authentication/session architecture; never compromise login security just to match the design.

## App navigation and labels

- **System quick controls:** no default visible labels.
- **Dock:** icon-first; names via tooltips/accessibility.
- **Launcher:** show names where necessary to distinguish apps/search results.
- **Settings:** concise labels are appropriate when they improve clarity.
- **Destructive, security-critical, ambiguous, and infrequent actions:** use explicit confirmation/explanatory text when necessary. Minimalism never overrides safety.

## App integration matrix to maintain

Keep an up-to-date implementation checklist with columns: `surface`, `module/path`, `existing component reuse`, `theme compliance`, `hover compliance`, `keyboard/a11y`, `backend wiring`, `tests`, `status`.

First inspect the repository and populate real paths. Do not guess source structure. Address P0 mismatches before creating cosmetic P2 features.

## Shared component governance

If two apps contain visually/behaviorally similar controls, consolidate them into the component library. Each migrated app should remove local duplicated rules and refer to central tokens. Update component documentation when functionality changes. Avoid app-local overrides unless they solve a real, documented requirement.
