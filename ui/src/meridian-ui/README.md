# Meridian UI Kit

Meridian's reusable React component API and visual system. Import from
`src/meridian-ui` rather than binding application pages directly to a UI vendor.
All controls are Meridian-owned React components built on standard HTML inputs,
buttons and ARIA patterns. There is no external UI kit, provider, stylesheet or
runtime component dependency. React remains the application framework.

## Color semantics

- macOS-inspired green (`#30d158` fill): enabled switches, sliders, selected dropdown choices, normal progress,
  healthy battery fill, new/unread notifications and positive actions.
- Blue: navigation selection, focus, links, secondary actions, connectivity and
  display category tiles.
- Orange: attention, startup/engine/storage categories and warning meters.
- Red: destructive actions, critical meters and notification/sound categories.
- White: neutral categories such as General, keyboard and peripherals.

Icon colors identify categories, not a strict severity ranking. Keep names,
On/Off labels, numeric percentages, accessible states and warning text so color
never carries the only meaning. Storage warns at 85% used and becomes critical
at 95%; battery warns at 20% remaining and becomes critical at 10%.

The grouped sidebar and category tiles take inspiration from
[Apple's System Settings organization](https://support.apple.com/guide/mac-help/change-system-settings-mh15217/mac)
and [semantic color guidance](https://developer.apple.com/design/human-interface-guidelines/color).
All icons, colors and application layouts remain Meridian's implementation.

## Components

- `Button`: default, primary (green), secondary (blue), danger and ghost;
  default/small/icon sizes. Native button props and disabled state are retained.
- `SurfaceButton`: layout-preserving action/menu/hover target. Supply `aria-label`
  for icon-only actions and an appropriate role when inside a real menu.
- `Container` / `Card`: bordered surface; accepts normal div props.
- `Section`: named content group with optional description and card body.
- `SettingRow`: consistent title, description, control and optional status.
  Associate its control using `label`, `labelledBy` and `describedBy` as needed.
- `Select`: single-choice themed listbox retaining existing option children and native select
  change-event/form semantics. Supply an accessible label or associated id.
- `Switch` / `Toggle`, `Slider`, `Checkbox`, `Radio`, `Progress`, `Tabs`, `Input`,
  `SearchField`, `Dialog`: shared interaction and state styling.
- `IconTile`: decorative category icon with a green/blue/orange/red/white tone.
- `Badge`: concise text status with a semantic tone.
- `HoverTip`: optional hover/focus hint; never put required instructions only
  inside a hint. Its child must accept ARIA props and have its own accessible name.
  Hints support hover/focus and Escape dismissal.
- `EmptyState`: title, explanation and optional recovery action.

```tsx
import { Button, Section, SettingRow, Switch } from "../../meridian-ui";

<Section title="Notifications">
  <SettingRow title="Notifications" description="Show new desktop alerts.">
    <Switch label="Notifications" checked={enabled} onChange={setEnabled} />
  </SettingRow>
  <Button variant="secondary" onClick={openDetails}>View details</Button>
</Section>
```

`tokens.css` is imported after legacy tokens in base.css. Each surface loads
`meridian-ui.css` after page styles so legacy CSS cannot redefine control
presentation accidentally. Component CSS is kept out of TypeScript imports to
avoid generated CSS filenames overwriting surface entry stylesheets. Use the
existing copy-static/release pipeline; no CDN or new build tool is required.

Reduced-motion and forced-color states are included. Popup menus remain inside
the current native WebView's bounds. This update was source-reviewed only;
rendering, contrast, keyboard behavior and layout still need live verification.

Settings path navigation uses clickable Settings/current-page breadcrumbs and a
Back button. Settings returns Home and clears the category search; Back restores
the previous visited page (up to 32 entries) with focus on its heading. External
page-opening events participate in the same history. The path shows page hierarchy,
not a fabricated chain of sibling pages.

Colored icon backgrounds have fixed equal dimensions and cannot inherit the
navigation label's growing flex behavior. Reduced corner radii keep them squarish.
Dropdowns keep keyboard focus on their combobox trigger, support arrows/Home/End,
type-ahead, Enter/Space selection, Escape/Tab dismissal and outside-click closing,
and skip disabled options. Native hidden selects retain form/change semantics.
Sliders use native range input keyboard/touch behavior; toggles are named buttons
with `role=switch` and `aria-checked`. Radio/checkbox controls retain native semantics.

## Alignment contract

Use a 4 px spacing base, 8 px compact gaps, 12 px normal gaps, 16 px panel/row
insets, 20 px card insets and 32 px page insets. Standard controls are 36 px high;
compact actions are 32 px. Use `--muk-*` spacing/alignment tokens instead of
introducing per-page measurements. Setting labels share a leading edge and
controls a trailing edge; descriptions stay with their labels. Numeric readouts
use fixed space and tabular digits. Settings/Pulse navigation shares a 232 px
sidebar, reduced on narrower windows. Breadcrumbs occupy their own row above
page headings. Native surface sizes and dock geometry are separate contracts.

SearchBar (`SearchField` alias) provides the shared search icon, clear button, focus ring and disabled styling. Pass `value` and `onValueChange`; input accessibility/keyboard props and `ref` reach the inner input, while `className` styles the wrapper. Use it for desktop search and filtering instead of composing input wrappers.

Switch uses a native checkbox with `role="switch"`; its wrapper owns the track and the input owns focus, labels and activation. Slider listens to native input events, preserves range keyboard behavior and commits once at the end of an interaction. Do not globally blur controls on pointer release.

## Feedback and navigation

- `Alert` accepts `tone="info" | "success" | "warning" | "error"`, an optional
  `heading`, content children and an optional `action`. Errors use the alert
  live region; other tones use status. Include meaningful status text.
- `Disclosure` accepts a text `title`, body children and native details props,
  including `open` and `onToggle`. Its native summary supports keyboard activation.
- `ActionTile` accepts `title`, optional `description`, an `icon` name and
  native button props such as `onClick` and `disabled`. It defaults to a
  non-submitting button and uses the shared tile radius.

These patterns use semantic surface and text tokens in both Light and Dark.
Settings consumes all three. Precision styles loaded after the kit define the
active palette and neutral icon treatment.

## Forms and confirmation

- `TextField` pairs an input with its label, help text and validation error using
  stable IDs, `aria-describedby` and `aria-invalid`. Native input props are forwarded.
- `PasswordInput` wraps TextField with a labeled show/hide button. It supports
  controlled or uncontrolled values and native autocomplete hints.
- `LoadingButton` takes `loading` and optional `loadingLabel`, disables activation
  during work and exposes `aria-busy`.
- `ConfirmDialog` accepts `open`, `title`, `confirmLabel`, `danger`, `onClose`,
  an asynchronous `onConfirm`, and descriptive children. It starts focus on
  Cancel, guards repeated activation, prevents dismissal during work, displays
  rejected promises inline and restores trigger focus on close.

Open Settings → System → UI Kit to exercise the components. Switch the theme
in Appearance to inspect both palettes. Preview actions modify preview state
only. Use Tab/Shift+Tab, arrows and Escape to inspect focus behavior.
