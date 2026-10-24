# Meridian UI reference analysis

> Nova and Vault have since been removed from the project. References below describe the earlier implementation.

Audit date: 2026-10-18. Inspected the actual React surfaces, shared controls,
CSS, theme propagation, native surface sizing, preview bridge and Phase 1–3
performance documentation. Existing wireless edits are retained.

## Reference access and decisions

No `~/meridian-ui-references` directory exists. Shell network DNS is unavailable.
The web reader accessed all five repository landing pages, libadwaita button
source, BreezeLight.colors and Fluent's token directory. Several individual
Granite, Fluent and FluentThemePlasma source URLs could not be retrieved; their
source-level analysis is incomplete. Do not treat inferred adaptations as a
complete review of those projects.

| Reference project | Useful design pattern | Meridian component / proposed improvement | Performance implications | Feasibility | License considerations |
| --- | --- | --- | --- | --- | --- |
| [libadwaita](https://github.com/GNOME/libadwaita), [button source](https://github.com/GNOME/libadwaita/blob/main/src/stylesheet/widgets/_buttons.scss) | Compact controls; distinct hover, checked, disabled and keyboard states | Shared toggles, Settings rows, dialogs: use semantic surfaces and visible focus | CSS only, no widget library | High within existing controls | LGPL-2.1+; no source or assets copied |
| [Breeze](https://github.com/KDE/breeze), [palette](https://github.com/KDE/breeze/blob/master/colors/BreezeLight.colors) | Separate window/view/button colors; semantic selection and focus | Dock, top bar, Orbit, Pulse: shared neutral chrome with blue selections | No blur or new renderers | High; retain native geometry | Mixed per-file licenses in LICENSES; no imported artwork/code |
| [Granite](https://github.com/elementary/granite) | Common GTK widgets and visual simplicity; adaptation inferred from project purpose, detailed source unavailable | Orbit: quiet sidebar, clear path and file canvas; reduce decoration | Retain thumbnail bounds and concurrency | High for styling; adaptive native patterns deferred | Check exact file license before any future reuse; none copied |
| [Fluent UI](https://github.com/microsoft/fluentui), [token structure](https://github.com/microsoft/fluentui/tree/master/packages/tokens/src) | Global/alias/theme token separation | Centralize dimensions, spacing, states and theme roles; reuse existing React controls | No Fluent runtime dependency | High; no component framework migration | MIT repository; no code/assets copied |
| [FluentThemePlasma](https://github.com/IviriusMain/FluentThemePlasma) | Breeze fork demonstrates desktop-wide style integration; detailed sources unavailable | Consistent dock, menu and application chrome, adapted to Meridian | Reject acrylic, large blur and extra compositing | High for CSS; no decoration engine changes | Per-file licensing must be checked before reuse; none copied |

## Surface audit

| Surface | Observed implementation issue | Implemented direction |
| --- | --- | --- |
| Desktop | Context menu colors layered over theme overrides; dense labels | Semantic menu states and calmer icon selection |
| Dock | Hard-coded theme backgrounds, running/active state mixed with hover | Shared opaque chrome, restrained shadow, distinct active underline |
| Top bar | Multiple icon shadows and theme-specific literals | Compact 32px geometry retained; semantic chrome and crisper icons |
| Launcher | Repeated layout overrides; competing radii | Retain fast search and keyboard model; unify search, tiles and menu shape |
| Control Center | Repeated blue literals and individually bordered cards | Shared primary color and subtle grouped surfaces; retain null hardware states |
| Settings | Long historical CSS layers and tiny secondary text | Shared row/control geometry, readable captions, consistent selection |
| Orbit (`finder`) | Toolbar/row sizes differ from settings; competing hover styles | Shared compact toolbar, calm navigation, explicit focus/selection |
| Terminal | Separate intentional ANSI palette | Keep renderer palette and metrics; align tab chrome only |
| Pulse | Local palette shadows shared ink/line tokens | Alias app colors to shared palette; retain charts and sampling cadence |
| Nova | Purple palette, light-only control styling, cramped long cards | Token-based cards, installed/action states and responsive grid |
| Calendar | Purple today/selection, faint dates, fixed-width agenda | Shared selection, readable month grid, responsive agenda |
| Clock | Purple controls and light background selected tab in dark mode | Semantic segmented tabs, stable tabular digits and primary actions |
| Notifications | Tiny timestamps and multiple card borders | Semantic surfaces and consistent cards; preserve history/actions |
| Login / Lock | 20–28px backdrop blur despite opaque cards | Remove blur; preserve dark authentication contrast and behavior |

## Constraints

Preserve bridge capabilities, authentication, file operations, native decorations,
88px panel allocation and 32px top bar allocation. Preserve lazy surfaces, ESM
sharing, cache limits, WebView reuse and visibility-aware timers. No new runtime
dependencies, continuous decorative animation or imported artwork. Visual quality
must still be reviewed on real WebKitGTK; build/test success is insufficient.
