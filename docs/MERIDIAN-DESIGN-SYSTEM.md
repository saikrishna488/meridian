# Meridian Design System

> This design-system audit is dated 2026-10-18. The current package is dark-only;
> the older Orbit and embedded Terminal surfaces described in this audit are
> not in the production UI bundle. See the [current README](../README.md).

The source of truth for shared tokens is `ui/src/lib/tokens.css`. The current
appearance hook fixes Meridian's own surfaces to dark mode; users can choose a
system font in Settings. Older light-theme token definitions and audit notes
remain in the source history, but light mode is not exposed by the current UI.

## Visual language

Cool neutral canvas, quiet navigation rails, clear blue selection, rounded
rectangular controls and solid floating shell surfaces define Meridian's visual
language. Accent is reserved for navigation and actions. Application artwork and
wallpapers remain Meridian's existing assets. Avoid acrylic, repeated outlines,
large diffuse shadows and decoration that competes with content.

| Role | Tokens / rule |
| --- | --- |
| Color | `canvas`, `surface`, `surface-raised`, `surface-subtle`, `surface-hover` separate content from chrome |
| Text | `ink`, `ink-muted`; `on-wallpaper` only over wallpapers |
| Interaction | `selection` + `selection-ink`; `action` for text; `button-primary` + `on-primary` for solid actions |
| Status | `danger`, `success`; always include text, never rely only on color |
| Type | Existing system font stack and saved font; 11.5/13/15/17/22px scale; 420/520/600 weights |
| Spacing | `space-1` through `space-8`: 4, 8, 12, 16, 20, 24, 32px |
| Geometry | 34px controls, 30px compact controls, 52px settings rows; larger rows allowed for descriptions |
| Shape | 8px controls, 12px cards, 18px popovers/dialogs, 24px large surfaces; pill only where appropriate |
| Elevation | `shadow-card` for small separation; `shadow-lg` only floating UI; `shadow-dock` fits the panel allocation |
| Borders | `line` for structural boundaries and form fields; use subtle surface separation inside cards |
| Icons | Existing SVG icon system; 18px navigation icons; retain app artwork and fixed aspect ratio |
| Menus | Shared palette, visible selected option and keyboard focus; Escape returns focus |
| Inputs | Shared text/placeholder colors; visible focus; disabled controls retain semantic disabled state |
| Switches | Existing React Toggle, 40×24px track, 20px thumb; theme primary color; transform-only thumb motion |
| Sliders | Existing range inputs and 100ms native-write debounce; retain unavailable readings and accessible labels |
| Dialogs | Existing focus traps, explicit cancel/confirm, semantic surfaces, bounded overflow |
| Motion | 120/180/240ms tokens; transform/opacity for movement; no perpetual decorative animation |

The existing `glass-*`, `accent`, hover and Settings aliases are compatibility
names pointing to semantic tokens. Do not create another literal palette in an
application. Terminal keeps its tested ANSI palette separately because terminal
foreground/background, cursor and selection must agree with xterm rendering.
Authentication cards remain dark for wallpaper contrast in both appearances.

## Implemented surfaces

- Dock/top bar: shared chrome, separate active/running indicators, compact native
  geometry, quieter shadows and crisp system symbols. Native title tooltips,
  pinning, drag/drop, multiple-window cycling and context actions are retained.
- Launcher, Control Center and Notifications: shared shapes, search focus, card
  surfaces and primary color. Hardware unavailable/disabled states are retained.
- Settings: one semantic palette behind compatibility aliases, shared row sizing,
  grouped surfaces, primary actions and switch styling. Existing wireless work is
  preserved.
- Pulse: shared theme aliases instead of a separate local palette. Graph data,
  process sorting and visibility-aware sampling are retained.
- Applications: installed-app search, dock pinning, desktop shortcuts and a
  shared confirmation dialog for supported package removal.
- Calendar: theme-aware date and agenda surfaces; arrow keys move by day/week,
  Home/End move within a week, and focus follows across month boundaries. Date
  labels expose event counts; a single date is in the Tab order.
- Clock: theme-aware segmented controls and primary actions, tabular responsive
  digits. Existing visible-only tick cadence is retained.
- Desktop icons: consistent caption sizing, restrained hover/selection. Login and
  Lock no longer request backdrop blur; authentication logic is unchanged.

## Accessibility

Keep labels, disabled states and focus rings visible. Shared styles respect
reduced motion, forced colors and increased contrast. Theme-specific token
specificity must not override high-contrast media rules. Calendar's month heading
announces navigation politely; its changing clock digits are not live regions.
Existing dialogs preserve focus trapping and restore focus on dismissal.

This is an incremental implementation, not a completed visual qualification.
The older surface CSS still contains historical geometry overrides; future cleanup
should follow rendered comparisons rather than a wholesale rewrite. See
`UI-DESIGN-VALIDATION.md` for exact checks, measurements and outstanding work.

## Reusable component foundation (2026-10-18)

The component audit is in [COMPONENT-SYSTEM-AUDIT.md](COMPONENT-SYSTEM-AUDIT.md).
`ui/src/components/ui/` owns the contributor-facing component API. These are small
React components using the existing CSS, following shadcn's editable-source model.
No generator, Tailwind configuration, second theme store or icon library is added.
**Base UI and Motion for React are not integrated yet:** npm registry access fails
with EAI_AGAIN, and neither package is cached. Do not treat this foundation as a
completed Base UI/Motion migration or native WebKitGTK qualification.

| Shared component | Contract and current use |
| --- | --- |
| `button.tsx` / Button | Native props and React 19 ref; defaults to `type="button"`. Explicit `type="submit"` for forms. Default/primary/danger/ghost variants and default/small/icon sizes. Settings, Control Center, confirmations, Orbit Properties, Nova, Calendar and Clock. |
| `switch.tsx` / Switch | Existing labeled checked/onChange contract and disabled button semantics; `components/Toggle.tsx` re-exports it for compatibility. Settings and Control Center. |
| `slider.tsx` / Slider | Native range input with clamped visual `--fill`; no hardware requests or state store. SystemSlider retains its 100ms debounce; Sound retains commit-on-release/key/blur. |
| `select.tsx` / Select | Re-exports the single existing Select implementation. Native select/change contract, labeled trigger and conditional body portal. Active highlight is separate from committed `aria-selected`. |
| `dialog.tsx` / Dialog | Controlled inline modal with named content, optional busy state and scrim/content class slots. Unmounts when closed; traps/restores focus; isolates siblings using reference-counted inert state. Drivers, Device Manager, Orbit Properties, shared uninstall confirmation, Nova. |
| `tabs.tsx` / Tabs | Controlled horizontal tabs; one tab stop; arrows/Home/End skip disabled items, respect RTL and activate/focus the next tab. Linked focusable panel remounts on selection. Clock modes. |
| `card.tsx` / Card | Semantic themed div shell; consumer owns padding/content structure. General Settings groups. |
| `checkbox.tsx` / Checkbox | Native input, consumer supplies its visible label. Orbit hidden-files control. |
| `progress.tsx` / Progress | Native progress element; omit value for indeterminate state. Orbit transfer bytes. |

Do not add speculative wrappers for every library primitive. Existing native menus,
scrolling, separators and title tooltips remain in use. DropdownMenu, ContextMenu,
Popover, Tooltip and ScrollArea need a concrete consumer and native qualification
before adding an alternative implementation. Native shell popovers are separate
layer-shell surfaces; a DOM Popover must never replace `popover.*` bridge calls.

### Styling conventions

`components/ui/components.css` is imported once by `lib/base.css` and uses semantic
tokens from `lib/tokens.css`. Default styles use `:where()` so application-specific
geometry continues to work. Explicit button variants have stronger selectors to
survive legacy surface button rules. Prefer component props for new code; retain
legacy classes during migration and remove them only after visual comparisons.
Use spacing/type/radius tokens, `line`, `shadow-card`/`shadow-lg`, and semantic color
pairs rather than new hardcoded palettes. Danger action fills use `button-danger`
and `on-danger`, distinct from `danger` status text. `layer-dialog` and
`layer-listbox` describe DOM stacking only; Rust controls native window stacking.
No library CSS reset is required. Appearance and font events remain unchanged.

### Animation patterns

`lib/motion.css` is the current shared CSS preset source: dialog entries use a
4px transform plus opacity over `dur-med`; tab panels and listboxes fade over
`dur-fast`. Every entry runs once, without retained `will-change`, layout animation,
backdrop blur, spring loops or delayed removal. Reduced motion disables these
animations. Simple button/switch feedback keeps existing CSS transitions.

Closed DOM dialogs/listboxes unmount immediately, so a transparent exit layer
cannot block input. Native menus preserve `usePopover`'s transition/close handshake;
while closing, their section is inert/aria-hidden and their scrim is hidden.
Do not substitute an exit animation for the native `popover.close` acknowledgement.

When package access is restored, evaluate `LazyMotion` with `m`/`domAnimation` only
at a consumer boundary, `MotionConfig reducedMotion="user"`, and a shared transition
preset derived from Meridian duration/easing tokens. Do not import Motion from
`mount`, the bridge, Dock or graph rendering. Test with the production CSP; do not
weaken it to accommodate animation. Compare the bundle report and native timings
before retaining the dependency. No Motion integration is claimed by the CSS file.

### Accessibility requirements

Give every icon button an accessible name, every field a visible label, and every
dialog an `aria-labelledby` or `aria-label`. Prefer explicit title and description
IDs for confirmation dialogs. Dialogs choose `data-autofocus`/autofocus among enabled
controls, otherwise the first enabled control, otherwise the dialog itself. Place
initial focus on Cancel for destructive actions. Escape only dismisses the top
focus scope; busy dialogs ignore it. Tab excludes hidden, disabled and negative
Tab-index controls, includes textarea, and wraps in both directions. Programmatic
focus outside a dialog returns inside; its owned listbox remains usable. Focus
restoration is conditional on the previous element still being connected/enabled.
Sibling inert state and background scroll are restored on unmount, including nested modals.

Select keeps keyboard navigation, disabled options, native change notifications
and form value; it exposes external/explicit labels and validation descriptions.
Its portal closes on Escape, selection, outside pointer, scroll outside the list,
resize, blur, visibility change and host popover dismissal. Escape in a listbox
must not dismiss the parent dialog. Check keyboard behavior in actual WebKitGTK;
jsdom cannot validate the native focus chain or screen-reader announcements.
Use native select directly for multiple/list selection; the shared Select API is
currently intended for single-choice controls.

### Usage

Import specific files; keep bridge requests in domain components:

```tsx
import { Button } from "../../components/ui/button";
import { Dialog } from "../../components/ui/dialog";
import { Switch } from "../../components/ui/switch";

<Switch label="Enable Wi-Fi" checked={enabled} disabled={unavailable}
  onChange={setEnabled} />

<Dialog open={confirming} onClose={() => setConfirming(false)} busy={saving}
  aria-labelledby="remove-title" aria-describedby="remove-detail">
  <h2 id="remove-title">Remove this item?</h2>
  <p id="remove-detail">Review this change before continuing.</p>
  <Button data-autofocus disabled={saving} onClick={() => setConfirming(false)}>Cancel</Button>
  <Button variant="danger" disabled={saving} onClick={remove}>Remove</Button>
</Dialog>

<Button type="submit" variant="primary" disabled={saving}>Save changes</Button>
```

For tabs, supply unique stable values and the active panel's content:

```tsx
<Tabs label="Mode" value={mode} onValueChange={setMode}
  items={[{ value: "clock", label: "Clock" }, { value: "timer", label: "Timer" }]}>
  {mode === "clock" ? <ClockFace /> : <TimerControls />}
</Tabs>
```

The panel remounts on selection; keep state that must survive tab changes above
Tabs, as Clock already does. For native progress, use
`<Progress aria-label="Bytes copied" max={total} value={copied} />`. Associate
Checkbox/Slider with a label or provide an aria-label; omit disabled hardware
writes in the owning domain component.

### Migration status

| Surface | This component pass | Remaining |
| --- | --- | --- |
| Settings | Buttons first; shared switch/range presentation; General cards; device/driver Dialogs; other modal users inherit improved focus helper | Account, startup, display-trial and wireless form wrappers; remaining card styles |
| Control Center | Shared buttons/switch/ranges; inert hidden section and hidden scrim | Retain native popover lifecycle; qualify native focus |
| Orbit | Properties Dialog/buttons, Checkbox, Progress; shared uninstall confirmation | File action forms/preview keep existing behavior; file grid/cache/hot paths retained |
| Nova | Buttons and package Dialog; existing Select improvements | Native authorization/focus validation |
| Pulse | Audited; existing accessible tabs and graphs retained | Only migrate with measured benefit; do not animate samples or process rows |
| Calendar | Ordinary action buttons consolidated; existing day-keyboard logic retained | Native visual qualification |
| Clock | Buttons and shared Tabs; timer/tick cadence retained | Native visual/keyboard qualification |
| Launcher | Shared uninstall Dialog; hidden surface input safety | Search/result/context menu hot paths retained |
| Dock | Audited; Dock implementation retained | No measured benefit for replacing native drag/drop/window grouping path |

### Validation and remaining work

The [component performance record](performance/component-system-2026-10-18.json)
contains actual baseline/after production bytes, per-surface transitive bytes and
unavailable runtime metrics. Reproduce after building with
`node ui/scripts/measure-bundles.mjs`. Bundle bytes exclude source maps; they do not
measure startup time, React render overhead, CPU or PSS.

Run `npm test --prefix ui` for production build/type check plus component, contrast
and existing bridge/surface tests. Run `tools/check.sh` for project checks and
`tools/tests/wayland.sh` for the native integration environment. In this sandbox,
D-Bus/socket creation is denied and GTK has no display. The native smoke suite
cannot run; renderer recovery, light/dark screenshots, reduced-motion rendering,
pointer stacking, layer-shell focus, native keyboard shortcuts and runtime
performance remain unqualified. Existing preview images are not after evidence.

Before declaring the migration complete:

1. Restore package access; install and lock only the Base UI primitives and Motion
   features actually exercised by migrated consumers. Use the upstream APIs and
   replace equivalent local behavior rather than maintaining duplicate controls.
2. Re-run production type/build/interaction tests and the bundle report. Keep
   MIT/other required notices with any copied shadcn code or vendored assets;
   preserve package LICENSE files in distributions and GPL-3.0-or-later for Meridian.
   This pass copies no upstream code and adds no runtime dependency.
3. In the isolated native Wayland suite, verify portals, focus trap/restore, nested
   Escape, stacking, stale popup cleanup, hardware unavailable/busy states, Rust
   bridge delivery and renderer crash/recovery. Never qualify these using jsdom.
4. Capture light/dark, reduced-motion, high-contrast and narrow-width screenshots
   using the existing fixture preview, then repeat inside real WebKitGTK.
5. Profile identical before/after native sessions using `tools/profile-desktop.py`:
   cold initial render, menu input-to-paint latency, idle/burst CPU, shell-tree PSS,
   individual WebKit renderer PSS and React commit counts/duration. Record hardware,
   build profile, monitor count, sample interval and open windows. Runtime fields
   remain null until actual measurements are available.


Actual checks for this pass: `npm test --prefix ui` passed all four files (111
individual cases). Release-integrity checks, rustfmt and clippy passed. The full
project script stopped on the existing greetd socket test with EPERM; a qualified
workspace run skipping only that test passed 211 tests, with 4 ignored and 1
filtered. The generated-types commit gate was not reached; those files were
already dirty before this pass. Native smoke binaries built, but the private
D-Bus session could not bind its socket. These limitations are recorded rather
than reported as passing native tests.

| Production asset bytes (unique files, no maps) | Before | After | Delta |
| --- | ---: | ---: | ---: |
| JavaScript | 725,691 | 730,032 | +4,341 |
| CSS | 137,458 | 141,664 | +4,206 |
