# Meridian UI refinement — October 2026

> Nova and Vault have since been removed from the project. References below describe the earlier implementation.

This pass prioritizes consistent desktop interactions and legible application artwork. It builds on the existing working tree without replacing its ongoing desktop features.

## Improvements

- Dedicated Drift compass and Vault safe icons; darker, higher-contrast Settings, Finder, Terminal, Nova, and Pulse artwork.
- Shared rounded icon silhouettes, clearer third-party icon tiles, and consistent artwork insets.
- Third-party icon resolution follows the installed GTK theme and includes user icons, Flatpak exports, host icons, and pixmaps.
- Missing desktop-entry artwork retries the theme icon before showing a colored initial, so failed images have a visible fallback.
- Legacy XPM artwork is converted to PNG before it reaches WebKit.
- Blue hover feedback across Finder navigation and tools, Settings navigation, desktop menus, application menus, dropdown options, and taskbar buttons.
- Finder file hover uses a selection tint and outline. The launcher uses calmer surfaces, consistent rounded tiles, and a blue search focus ring in light and dark themes.
- Shared native application chrome for Drift and Vault: rounded controls, blue button hover, entry focus rings, tab selection, and progress indicators.
- Drift adds a recent-history menu (200 unique recent pages retained, 50 shown), history clearing, and zoom reset. Private sessions do not record browsing history.
- Drift adds previous/next find controls, a close-find button, and Enter-to-next-match. Reload changes to Stop while loading; navigation starts disabled; closing the final tab closes the window.
- Vault adds volume cards, drive symbols, storage usage bars, device-path copying, and filtering by visible name, device path, filesystem, or mount point. Filters remain applied after refreshing or mounting.
- Vault formats small capacities in bytes/KB/MB and accepts numeric-string capacities from lsblk.
- Existing system-volume unmount protection remains covered by tests.

## Validation

- UI production build, TypeScript checking, and both UI test suites.
- Regression coverage for missing-icon fallback and dedicated Drift/Vault artwork.
- Drift unit tests: address routing and bounded, deduplicated recent history.
- Vault unit tests: capacity formatting and protection of system/mixed mounts.
- URI scheme tests: asset access, path confinement, and locker preflight.
- Clippy for shell binaries with warnings denied; diff whitespace checks.

The XPM conversion integration test requires image-loader D-Bus access. It is explicitly ignored in the default suite; run it in a normal desktop environment with:

```sh
cargo test -p meridian-shell --lib legacy_xpm_icons_are_encoded_as_png -- --ignored
```

Live screenshot review remains outstanding: this execution sandbox blocks the local preview server's listening socket, and the image loader's D-Bus connection. These changes have not been installed into or visually certified in a running Meridian session. They do not establish a usability comparison with other desktop environments.
