Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# UI design validation — 2026-10-18

> Historical validation record for the October 18 design pass. Later code
> changes made the UI dark-only and removed Finder/terminal surfaces from the
> production bundle. The current screenshot checklist is at the end of this
> document; earlier capture references above are not current feature claims.

> Nova and Vault have since been removed from the project. References below describe the earlier implementation.

See [reference audit](UI-REFERENCE-ANALYSIS.md) and
[Meridian Design System](MERIDIAN-DESIGN-SYSTEM.md).

## Scope and preserved behavior

This pass changes UI styling, adds Calendar keyboard date navigation and reuses
the category Select in Nova. It adds no dependencies and changes no Rust, bridge
permissions, file operations, PAM/locking, window management or hardware polling.
The pre-existing wireless implementation and generated protocol changes remain
in the working tree; they are not part of this design pass.

The production ESM graph, renderer isolation, lazy window creation, retained
WebViews, bounded caches, visibility-aware Clock/Pulse timers and debounced slider
writes remain. Calendar's event dots now use one event-count map per render
instead of scanning all events once per day cell. No speedup is asserted.

## Executed checks

- `npm test --prefix ui`: production TypeScript/build and UI interaction suites,
  including Calendar focus across month boundaries, Nova installed filtering
  without package mutation, and light/dark semantic contrast checks.
- Contrast tests cover text/actions at 4.5:1 on four surface roles, selected text,
  primary button text including hover, focus at 3:1 and switch-thumb contrast at
  3:1. They caught a low-contrast dark primary hover; its color was corrected.
  These checks do not establish accessibility of every legacy literal color.
- `tools/check.sh`: release integrity, screenshot-command fixtures, power/session
  tests, installation rollback, profiler tests, rustfmt and full workspace Clippy
  passed. Workspace tests stopped at
  `greetd::tests::interrupted_and_malformed_socket_responses_fail_cleanly` because
  the sandbox rejects socket creation with `Operation not permitted`.
- Qualified native run:
  `cargo test --workspace -- --skip greetd::tests::interrupted_and_malformed_socket_responses_fail_cleanly`:
  211 passed, four already-ignored tests, one explicitly filtered socket test.
  This does not count the socket test as passed.
- `tools/tests/wayland.sh`: native integration binaries built; private D-Bus
  failed to bind `/tmp/dbus-*` with `Operation not permitted`, before compositor
  testing. Nested integration behavior is unverified here.
- `git diff --check`: passed.

`tools/check.sh` also requires generated TypeScript to match committed files.
Those files already contain the user's uncommitted wireless work; no commit or
revert was made to satisfy that gate. A fully green check remains outstanding.

## Measured build footprint

Same production build command and dependency installation, baseline captured
before design edits (including the pre-existing wireless working tree). Sum each
unique file under `ui/dist` by extension; exclude source maps. These are uncompressed
build bytes, not per-window heap or physical memory.

| Metric | Before | After | Difference |
| --- | ---: | ---: | ---: |
| JavaScript bytes | 724,649 | 725,691 | +1,042 (+0.14%) |
| CSS bytes | 137,643 | 137,458 | −185 (−0.13%) |

[Machine-readable record](performance/ui-design-2026-10-18.json).
No before/after runtime PSS, idle CPU, GPU cost or launch latency is available.
The older Phase 2 measurements describe a different change and are not reused as
results for this pass. Repeat the Phase 2 profiler procedure on the target laptop
with matching renderer, monitor count, build profile and open applications.

## Visual validation limitation

No new screenshots were generated. `node tools/preview-ui.mjs` failed to listen
on `127.0.0.1:4173` with `EPERM`. A private headless Wayland session also failed
because D-Bus cannot bind its socket. GTK's default GDK display is `None` in this
execution context. Existing images in `docs/previews` predate these changes and
are not presented as after screenshots. No claim of rendered visual completion
or absence of layout regressions is made.

## Remaining visual and hardware qualification

On an unrestricted desktop, build with `npm run build --prefix ui`, then run
`node tools/preview-ui.mjs`. Use the read-only fixtures; do not change real
hardware or files for screenshot staging. Preview paths below are relative to
`http://127.0.0.1:4173`. Capture the current dark appearance; do not present the
historical light/dark variants below as current.

| Capture | Preview path |
| --- | --- |
| Desktop | Native labwc capture with Wallpaper, Panel, and Topbar composed |
| Launcher | `/ui/surfaces/applications/index.html` |
| Control Center | `/ui/surfaces/options/index.html` |
| Settings Home and categories | `/ui/surfaces/settings/index.html` |
| Pulse | `/ui/surfaces/pulse/index.html` |
| Calendar | `/ui/surfaces/utilities/index.html?app=calendar` |
| Clock | `/ui/surfaces/utilities/index.html?app=clock` |
| Notifications | `/ui/surfaces/notifications/index.html` |

GNOME Files and Kitty are separate host applications; capture them only as
external windows in a native desktop session, not as Meridian web surfaces.

Capture 1280×800, 720px and 420px widths; check long filenames, app descriptions,
empty/unavailable states, focus, selected dates, overflow, high contrast and
reduced motion. Compare against a baseline from the same fixture/build settings.
Run `bash tools/tests/wayland.sh` in its isolated session, then validate actual
unlock, hardware controls, dock drag/drop, fractional scaling and native window
geometry on the Intel i5-8250U target. The original authentication, capability
checks and file safety paths are unchanged, but live revalidation is outstanding.
