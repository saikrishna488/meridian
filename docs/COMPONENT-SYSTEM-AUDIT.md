# Component system audit

> Nova and Vault have since been removed from the project. References below describe the earlier implementation.

Audited 2026-10-18 before component changes. Baseline includes the existing,
uncommitted appearance and wireless work; those changes are preserved.

## Existing architecture and reuse

React 19 + TypeScript, esbuild ESM splitting, fourteen independent WebView entries.
`ui/package.json` has only React/ReactDOM runtime dependencies; esbuild, TypeScript,
React types and jsdom are development dependencies. No Tailwind, Radix, Base UI,
shadcn runtime, animation package, CSS-in-JS or class utility is installed.

| Location | Reusable behavior / decision |
| --- | --- |
| `components/Toggle.tsx` | Labeled, disabled button switch with CSS thumb transition. Keep behavior and compatibility export. |
| `components/SystemSlider.tsx` | Range control + 100ms hardware-write debounce + unavailable state. Preserve bridge and debounce; extract only native range presentation. |
| `components/Select.tsx` | Native form/change semantics plus positioned portal listbox. Preserve API; fix label association, selected/active distinction and dismissal. |
| `components/ConfirmButton.tsx` | Three-second destructive-action confirmation and busy guard. Preserve. |
| `components/UninstallDialog.tsx` | Shared Finder/Launcher removal flow; manual button-only trap does not restore trigger focus. Consolidate modal behavior. |
| `components/PasswordField.tsx` | Reveal password/caps-lock feedback. Preserve. |
| `components/{Icon,AppIcon,BatteryIcon,MeridianLogo}.tsx` | Existing SVG/artwork system. Preserve; no icon dependency. |
| `components/{Clock,Dock}.tsx` | Native-window grouping/activation and visible clock cadence. Keep hot paths. |
| `lib/react.tsx` | Bridge-aware mount, native popover generation guards, callback refs, dialog/menu focus helpers. Retain native lifecycle. |
| `lib/{appearance,desktop,settings,power,apps}.ts` | Existing state/subscription boundaries. Do not add a second theme/state store. |
| `lib/{tokens,base,menu}.css` | Semantic light/dark tokens, focus/reduced-motion/high-contrast styles; extend these rather than replacing configuration. |

## Duplication, styling and accessibility gaps

- Settings actions, Control Center actions, Orbit properties, Nova and utilities
  repeat ordinary button markup and control styles. Consolidate buttons first;
  leave large search result lists, file grids, graphs and dock items alone.
- Device/driver, account, startup, Orbit and Nova dialogs repeat scrims, labels,
  focus setup and dismissal. Initial focus is often safe, but the existing helper
  misses textarea, hidden/negative-tabindex controls and empty-dialog fallback.
  It does not contain programmatic focus or coordinate nested dialogs.
- The custom Select labels only its trigger through a computed string. Wrapping
  labels and aria-labelledby need explicit support. It reports the active cursor
  as selected even before committing. A portal can survive host dismissal/window
  blur; scroll/resize positioning must not leave a stale popup.
- Clock uses pressed buttons for modes; Pulse separately implements roving tabs.
  Consolidate tab navigation without touching sampling, graphs or timers.
- Cards and range controls repeat presentation; native progress and checkboxes
  already have correct semantics and need only thin shared wrappers.
- Historical surface CSS repeats radius/padding/color declarations and overrides
  earlier rules. Avoid broad deletion until rendered comparisons are available.
- Reduced motion already zeroes duration tokens. Existing animation uses CSS
  opacity/transform for shell menus, switch thumbs and panel states; audit with
  `rg 'transition:|animation:|@keyframes' ui/src`. Preserve native close sequencing.
- Native shell menus stay mounted while hidden. Hidden menus/scrims must reject
  pointer input and keyboard focus immediately, including while closing.

## Library evaluation

[shadcn/ui](https://ui.shadcn.com/docs) supplies editable composable source.
Adopt that ownership/composition approach with Meridian CSS; do not run its CLI
or copy Tailwind defaults. [Base UI](https://base-ui.com/react/overview/quick-start)
is unstyled and tree-shakable. Its Dialog/Tabs are candidates for a later measured
replacement of local behavior, using component subpath imports and local portal
containers. A whole collection or second switch implementation is unnecessary.
[Motion](https://motion.dev/docs/react-reduce-bundle-size) should use a lazy feature
boundary only in surfaces that need coordinated presence, never in shared mount
or the dock. First compare its added transitive bytes and native timings.

The npm registry is unreachable (`EAI_AGAIN`, including an explicit retry with
retries disabled). Neither Base UI nor Motion is cached locally. They cannot be
installed/locked/tested here. No unresolved dependency, fabricated adapter or
vendored approximation is added. The foundation below uses existing React/CSS;
actual Base UI and Motion integration remains outstanding and is not claimed.
No upstream component source is copied, so no new upstream code notices apply.

## Performance boundaries and baseline

Preserve Launcher search debounce/stale-response guards, Orbit thumbnail/cache
bounds and transfer events, Pulse visibility-aware 2s polling and 31-frame cap,
Clock visible-only ticks, terminal xterm/PTY scheduling, Dock grouping/drag/drop,
and `usePopover`'s generation and native close handshake. Do not animate list
rows, graph samples, dimensions, blur or continuously running decorations.

Baseline `npm test` passed all three test files. Production bundle: 725,691 unique
JS bytes and 137,458 CSS bytes, excluding source maps. Per-surface transitive
bundle measurements and final comparison are in the component-system performance
record. These are build sizes, not memory or runtime latency.

GTK 4.22.4, WebKitGTK 2.52.6 and gtk4-layer-shell 1.3.0 are installed. Native
validation was attempted: `Gdk.Display.get_default()` returns None and private
D-Bus fails to bind its socket with EPERM. Native behavior, renderer recovery,
visual comparisons and CPU/PSS/latency cannot be qualified in this sandbox.
