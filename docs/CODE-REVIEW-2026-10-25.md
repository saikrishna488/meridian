# Meridian code review — 2026-10-25

Reviewed the current working tree, including existing uncommitted changes.
This review used source inspection and automated checks; it did not run an
installed graphical session or install packages on the host.

## Remaining findings

1. **High: dock context menus use coordinates from the wrong window.**
   [Dock.tsx](../ui/src/components/Dock.tsx#L103) sends a button's viewport-relative
   `getBoundingClientRect().top` from the 64-pixel panel WebView.
   [desktop-menu.tsx](../ui/src/surfaces/desktop-menu/desktop-menu.tsx#L60) uses that
   value as a position in the much taller overlay WebView. A value near 10 pixels
   places the menu's bottom near the overlay's top, leaving most of its actions
   outside the visible area. Right-click a dock application to exercise this path.
   Translate coordinates between surfaces, or anchor this menu above the panel
   using the overlay's own bottom edge.

2. **Medium: login overwrites the user's default browser.**
   [meridian-session](../session/meridian-session#L19) unconditionally changes HTTP,
   HTTPS, and HTML associations whenever Firefox is installed. A user who selects
   another browser loses that selection on the next Meridian login. Preserve
   existing associations or make this an explicit preference.

3. **Medium: keyboard context menus expose app actions for non-app results.**
   [applications.tsx](../ui/src/surfaces/applications/applications.tsx#L83) handles
   Shift+F10 and the Context Menu key without checking the selected provider.
   Search for `2+2`, select the calculator result, then press Shift+F10: it offers
   pin, desktop shortcut, and uninstall operations using the expression as an app
   ID. Settings results have the same problem. The mouse handler already checks
   `provider === "apps"`; apply that condition to keyboard activation too.

4. **Medium: the UI test suite still targets removed bundles and controls.**
   [build.mjs](../ui/scripts/build.mjs#L10) excludes Finder and Terminal, while
   [desktop-features.test.mjs](../ui/tests/desktop-features.test.mjs#L95) still loads
   them from `dist`. Other assertions expect removed dock Trash, launcher list and
   pinned views, and light-mode controls. Direct runs produced 28 failures out of
   70 desktop-feature tests and 4 failures out of 29 surface tests. Update tests
   for the external app integration and current UI; these counts do not represent
   32 independently established runtime bugs.

5. **Medium: the bridge permission regression test is out of sync.**
   [bridge.rs](../shell/src/bridge.rs#L154) expects only Wallpaper and Finder to
   authorize `desktop.add_file` and `desktop.remove`, but DesktopMenu now has
   `DesktopFiles`. The test fails deterministically on DesktopMenu. Decide the
   intended permission boundary and align the capability definition and test.
   This mismatch alone does not establish an exploitable authorization bug.

6. **Medium: the required Clippy check fails on an obsolete helper.**
   [surfaces.rs](../shell/src/shell/surfaces.rs#L5) retains `open_utility` after
   Calendar and Clock launch external apps. It has no callers, so the required
   `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails
   with `dead_code`. Remove the unused helper and review the remaining legacy
   surface state.

## Addressed during installer cleanup

The Debian package declared only five application dependencies, omitting the
compositor, WebKitGTK, layer-shell, lock tools, and other required programs.
Its dependency field now uses the shared runtime and codec lists, with minimum
versions for GTK, WebKitGTK, and layer-shell. It now includes the greeter
configuration as well. Package contents and dependencies are verified by
`tools/tests/build-deb.sh`.

Removed the redundant self-extracting installer scripts and source-install
wrapper. Debian packaging is the documented distribution path; `install.sh`
retains direct source installation and optional login-screen controls.

## Check results

- UI TypeScript check and bundle build passed; `npm test` failed in the two
  suites described above. Component and design-system suites passed.
- `cargo test --workspace --offline`: protocol 123 passed; services 21 passed;
  shell 63 passed, 2 failed, 4 ignored. One failure is the permission-test mismatch
  above; the other is the greetd Unix-socket test, which the sandbox blocked with
  `Operation not permitted`.
- Clippy failed on the unused `open_utility` method.
- Installer cleanup: Debian package fixture inspection, release integrity,
  source-install rollback, shell syntax, and `git diff --check` passed.
- No privileged installation, package removal, or real hardware tests were run.
