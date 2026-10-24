# Meridian — Design System & UI Implementation Contract

> **Status:** Mandatory design contract for every Meridian desktop surface.
> **North star:** A coherent, refined, compact, dark-only desktop environment that feels like one deliberately designed product—not a collection of unrelated apps.

## 1. Non-negotiable rules

1. **Dark mode only.** Remove light-mode UI, light-mode switching, automatic light/dark switching, alternate light themes, and dead theme code/configuration once it is safe to do so. Do not silently reintroduce light-mode styles through system preferences or third-party defaults.
2. **Pure black foundations.** Primary desktop, window, panel, sidebar, popover, launcher, dock, modal, and app backgrounds must be `#000000` unless a documented overlay/surface needs a clearly justified variant. Prefer borders, spacing, and opacity over introducing gray fill panels. Transparency, where appropriate, must composite over black and retain the black visual identity.
3. **Text colors are exclusively white or black:** `#FFFFFF` or `#000000`. This applies to body text, headings, button text, labels, shortcuts, placeholder text, selected states, hover states, focus states, and disabled states. Never use gray, green, blue, or any other hue for text. When muted hierarchy is required, change opacity of white/black rather than its RGB values—while preserving readability.
4. **Hover never changes text to a color other than white or black.** Choose whichever provides legible contrast against the hover background. Do not sacrifice readability to maintain a decorative effect.
5. **Single unified theme.** Every Meridian-owned surface imports shared design tokens and components. No app-specific color systems, font scale inventions, shape languages, toggle designs, or inconsistent spacing.
6. **Compact and icon-first.** Avoid visible labels when the icon and position are understandable. Icon-only controls MUST still have accessible names, keyboard access, and tooltips (and optionally a short contextual hint on long-press).
7. **Small, uniform icons.** Use consistent vector icon size, optical weight, padding, and alignment. Never mix giant emoji, photographic images, arbitrary glyphs, and mismatched line weights in system controls.
8. **macOS-inspired icon treatment, not a clone.** App/control icon tiles may use restrained, solid backgrounds chosen from **blue, white, gray, green** (plus transparent where the component does not use a tile). Colors are for icon containers, hover fills, indicators, and decorative visual cues—not text. Icon glyphs must be either white or black. Consistent geometry and predictable size matter more than novelty.
9. **Reuse components everywhere.** If the same interaction appears twice, it belongs in the shared component library. No second implementation of a toggle, segmented control, slider, menu, button, tooltip, icon button, settings row, dialog, or focus ring without an explicit exception.
10. **Functional, not mock.** Beautiful visuals must accompany working state, correct OS actions, reliable persistence, input accessibility, and appropriate error handling.

## 2. Theme token specification

Create a single `theme/tokens` source of truth (adapt to the repository's architecture); all frontend surfaces must reference it.

```css
:root {
  color-scheme: dark;
  --bg: #000000;
  --surface: #000000;
  --surface-elevated: #000000;
  --text-primary: #FFFFFF;
  --text-inverse: #000000;
  --icon-white: #FFFFFF;
  --icon-black: #000000;
  --icon-tile-blue: #247CFF;
  --icon-tile-green: #28A66A;
  --icon-tile-gray: #60636A;
  --icon-tile-white: #FFFFFF;
  --border-subtle: rgba(255,255,255,.14);
  --border-strong: rgba(255,255,255,.28);
  --text-muted-opacity: .72;
  --radius-control: 10px;
  --radius-panel: 14px;
  --radius-icon: 9px;
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --icon-size: 16px;
  --icon-tile-size: 28px;
  --control-size: 32px;
  --motion-fast: 120ms;
  --motion-standard: 180ms;
}
```

The values above are **initial defaults**, not permission for arbitrary exceptions. Audit and refine as a cohesive system, then update tokens rather than hard-coding local values. Pure black applies to main backgrounds; solid hover and icon tile colors are intentionally allowed. Render text only as white/black (opacity permitted).

## 3. Hover background customization

Expose a **desktop-wide hover background preference** with four options:

- `auto` — select a deterministic, accessible hover background appropriate to the control/context; never assign a random new color on each hover.
- `green` — use the shared green background token.
- `blue` — use the shared blue background token.
- `grey` — use the shared gray background token (UI spelling must be **Grey**).

Implementation requirements:

- Store preference centrally and persist it across sessions.
- Apply to compatible buttons, icon buttons, menus, list entries, dock items, launcher results, quick-settings toggles, and other interactive surfaces.
- Use one shared hover-state utility/component API, not app-specific selector overrides.
- Ensure white or black text/icon glyphs maintain adequate contrast on every option; select automatically per background. Focus, pressed, selected, disabled, and keyboard states must remain distinguishable.
- Hover animations must be subtle, fast, and non-distracting. Honor reduced-motion preferences.
- Do not let a hover setting recolor the desktop's pure-black foundation.

## 4. Component contract

Build and document shared components such as `Icon`, `IconTile`, `IconButton`, `Button`, `Toggle`, `Switch`, `Slider`, `Popover`, `Tooltip`, `Menu`, `MenuItem`, `SearchField`, `Panel`, `Dialog`, `AppCard`, `WindowControls`, `Divider`, `ShortcutHint`, `StatusIndicator`, `QuickSettingTile`.

Each component must define consistent sizing, tokens, props, interaction states, keyboard navigation, accessible names, and tests. Prefer composition over multiple almost-identical variants.

**Sizing defaults:** icon glyph `16px`; icon tile `28px`; compact control target visually around `32px`, with an enlarged interactive hit target when needed for usability; consistent 4/8px grid. For touch-enabled devices ensure comfortable pointer/touch targets without making the visible UI bulky.

**Hierarchy:** use layout, size, weight, spacing, borders, and opacity instead of multicolored text. Prefer 1–2 text sizes per compact panel. Avoid unnecessary headers and decorative dividers.

## 5. Control Panel / Quick Settings — redesign priority P0

Redesign the control panel as a **small, dense, instantly scannable grid of toggles**, not a settings application squeezed into a popup.

- **No visible labels in the main toggle grid.** No “Wi-Fi”, “Bluetooth”, “Night Light”, etc. beside or below every icon. Use recognizable small icons; accessible names and tooltips remain mandatory.
- Toggle tiles share identical visual dimensions and component logic. On/off differences must be visually perceivable without relying on hue alone (e.g. outline, filled state, indicator, icon treatment).
- Present only high-frequency controls by default; secondary actions appear through context menus, expansion, or a dedicated settings screen.
- Preserve fast access to Wi-Fi/Bluetooth/network/audio/display state, as supported by Meridian's actual backend. Do not show controls that cannot function.
- Make volume/brightness controls compact, potentially in an expandable row, using the same slider component.
- Position, padding, animation, borders, corners, and hover effects must match the launcher, dock, status popovers, and settings.
- Keyboard navigation, Escape-to-dismiss, screen readers, focus management, and visible focus states are required even when labels are hidden.
- Empty/error/loading and unavailable-hardware states must not produce misleading on/off displays.

## 6. Icons

- Maintain one shared icon library and mapping for system actions and app identities. Prefer vector assets; use SVG/component assets where supported.
- Use a **single icon size and visual weight** within each context; system quick controls should look uniform.
- macOS-inspired rounded-square backgrounds use one **solid color per tile** from blue/white/gray/green. A stable mapping by app/action identity may look varied; truly random recoloring between renders is prohibited.
- Use contrasting white or black glyphs. Avoid gradients, rainbow palettes, neon glows, skeuomorphism, or large shadows.
- Keep decorative icon tile color separate from shared interaction/hover preference.
- Third-party app icons may retain brand marks in the launcher where necessary for recognition, but Meridian-owned controls must follow this contract.

## 7. Desktop-wide visual consistency audit

Apply this design contract to: desktop/workspace, wallpaper fallback, top bar/status bar, dock/taskbar, app launcher/search, window chrome/titlebars, menus, context menus, notifications, control panel, settings, lock/login surfaces if Meridian owns them, dialogs, task switcher, file/browser apps, and system utilities.

For each surface verify:

- Background is black and typography is white/black only.
- Icons follow shared sizing/alignment and tile rules.
- Buttons, toggles, sliders, popovers, and tooltips are reused.
- Hover mode works; keyboard focus and selected states are coherent.
- No isolated light areas, stray theme variables, inconsistent corners, or oversized labels.
- At common screen sizes and scaling settings, no clipping or accidental tiny tap targets.

## 8. Motion and performance

Target crisp animations with restrained transitions and no excessive blur/backdrop-filter work. Prefer compositor-friendly transforms and opacity; keep main layouts stable. Optimize for older/low-end GPUs as well as modern machines. Respect `prefers-reduced-motion` (or platform equivalent).

## 9. Acceptance criteria

A change is design-complete only when it: (a) uses global tokens, (b) reuses shared components, (c) follows dark-only/black background/white-or-black text, (d) respects hover settings, (e) has meaningful keyboard and accessibility behavior, (f) has working backend state, and (g) has visual review against at least one other desktop surface.
