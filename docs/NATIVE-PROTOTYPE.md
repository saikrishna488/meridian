# Swift desktop prototype

The native prototype uses SwiftCrossUI/GTK4 and C++20, with no Rust or WebKit
runtime. It can run alongside the installed desktop or as the shell of a separate
labwc session. The production desktop remains on the existing stack until parity
and authentication qualification are complete.

## Build and run

Use Swift 6+, CMake 3.22+, GIO Unix 2.78+, libsystemd 257+, GTK 4.18+,
gtk4-layer-shell 1.2+, and a C++20 compiler. SwiftPM downloads pinned dependencies.

```sh
MERIDIAN_BUILD_CONFIGURATION=debug MERIDIAN_NATIVE_TESTS=1 bash tools/build-native.sh
bash tools/dev-native.sh
# Separate compositor, nested when launched from a graphical desktop:
bash tools/dev-native-session.sh
```

Super+Q exits the test compositor; Alt+Tab switches application windows.
The session starts only the native prototype, not meridian-shell or the web UI.
The standalone executable accepts `--desktop` for the workspace, `--desktop
--session` to add desktop surfaces, or `--overlay` for the keyboard launcher.

## Available workflows

- Search and launch installed applications, refresh the application catalog, and
  show launch failures.
- Navigate directories by path, parent, or Home; refresh listings and include
  hidden files; create directories, preview text, and open files in default apps.
  Listings and previews retain the C++ backend's size bounds.
- Load and save existing search preferences and inspect OS, host, architecture,
  kernel and memory information. Preferences use the existing per-user config
  format. Saving preferences does not implement global desktop search.
- Session mode provides a layer-shell wallpaper, top bar, and dock. Bars reserve
  their actual heights. Their navigation buttons select and reopen the workspace.
  Desktop and bar surfaces take keyboard focus on interaction.

Blocking backend calls run on a Swift actor with separate capability-scoped
clients for Files and Settings. Requests have a deadline, bounded native worker
admission, copied JSON replies, and deterministic native handle cleanup. Errors
remain visible in the corresponding pane.

## Checks

```sh
bash tools/check-native.sh --wayland
MERIDIAN_BUILD_CONFIGURATION=debug MERIDIAN_NATIVE_TESTS=1 bash tools/build-native.sh
python3 tools/tests/native-wayland.py \
  --launcher target/swift-ui/debug/meridian-native-applications
python3 tools/tests/native-workspace.py --screenshots target/native-workspace
```

The Swift integration test uses private temporary files/configuration and checks
folder creation, directory listing, text preview, settings persistence across
runtime reopening, denied capability access, and unknown surface rejection.
C++ surface tests cover overlay, wallpaper, top bar and dock roles, keyboard
policy, anchors, exclusive zones, idempotence, mapping and teardown.

For a project-local extracted Swift toolchain, add its real compiler directory
to PATH and its runtime directory to both LD_LIBRARY_PATH and LIBRARY_PATH when
building. `dev-native.sh` recognizes the existing local Fedora runtime layout.
Do not ship that development toolchain as part of the desktop package.

## Verified in this checkout

A Linux/aarch64 debug build with Fedora Swift 6.3.2 passes the 14 backend/catalog/
ABI contracts, Swift service integration test, and the isolated Wayland surface
contract. Virtual keyboard checks pass for launcher search/refresh/Return/Escape,
Files folder creation, saving both search preferences, and starting all Swift
session surfaces with fatal GTK critical checks. Workspace and session screenshots
were inspected. Physical input, accessibility and optimized release builds remain
unqualified.

## Remaining before production replacement

This is a testable shell prototype, not full migration parity. Multi-monitor
surface ownership/hotplug, task/window tracking, dock drag reordering, Control Center,
complete Settings, notifications, terminal/PTY, lock/greeter, installer/packaging,
and physical-input/accessibility/release qualification remain. Session mode hosts
surfaces on the compositor-selected output. Closing the test compositor is not a
screen lock. The authentication path must use a compositor session-lock protocol
and PAM/greetd before any production switchover.

## macOS-style Swift component kit

The shell now uses the reusable `MarsOSKit` Swift library, adapted from Luminare's
surface and component architecture with BSD-3-Clause attribution. See
[kit components and test commands](../swift-ui/KIT.md). It supplies native icon dock
items, slim top-bar styling, grouped sections and actions. Desktop right-click
includes Files, New Folder, Refresh, wallpaper choices and Settings. Application
pins and wallpaper palette choices are persisted for the native shell.
