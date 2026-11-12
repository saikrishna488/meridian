# MarsOSKit

A reusable Linux Swift component library adapted from Luminare's surface, button
and grouped-section design architecture. The adaptation targets the existing
SwiftCrossUI/GTK4 renderer and C++ platform bridge. Upstream source revision and
BSD-3-Clause notices are recorded in `THIRD-PARTY.md` and `Licenses/`.

## Shared components

- `MarsTheme`: adaptive canvas, surface, accent, secondary text and corner tokens.
- `.marsMaterial(...)`: chrome, floating dock, grouped section and panel treatments.
- `MarsActionButton`: neutral and primary actions, keyboard focus and disabled states.
- `MarsDockItem`: 44px installed-theme artwork, hover/pressed states, accessible
  names and native tooltips, used for built-in and user-pinned applications.
- `MarsDesktopItem`: desktop icon and label with an accessible control target.
- `MarsSection`: quiet heading and grouped content surface.
- `MarsSymbol`: GTK icon/GIcon adapter; keeps Linux app artwork recognizable.

Native styles are installed once per display, scoped to kit classes, and follow
GTK's light/dark theme colors. `.marsAppearance()` aligns Swift tokens with
the desktop portal and explicit GTK_THEME overrides, and requests the same
appearance from native GTK widgets. The renderer owns CSS hover, pressed, disabled and
focus states; Swift owns view state and actions. The kit does not run animation
polling timers. The desktop clock updates every 15 seconds while visible.

The desktop uses a 32px top bar and a bottom-centered icon dock with an inset.
The pointer-positioned context menu provides Open Files, New Folder, Refresh,
Change Wallpaper and Settings. Folder names go through the existing C++ filename
validation. Desktop and bar surfaces request keyboard focus on interaction; popup dismissal
releases the native menu grab.

Materials use tinted alpha surfaces. Compositor backdrop blur, SF Symbols and
Apple-only AppKit visual effects are not implemented. Pin order and wallpaper
selection are stored under the native shell's `UserDefaults` keys; this does not
change the old React desktop's dock preferences.

## Run and verify

```sh
MERIDIAN_BUILD_CONFIGURATION=debug MERIDIAN_NATIVE_TESTS=1 bash tools/build-native.sh
bash tools/dev-native-session.sh
bash tools/test-native-desktop.sh --screenshots target/native-chrome
GTK_THEME=Adwaita:dark bash tools/test-native-desktop.sh --screenshots target/native-chrome-dark
```

The test runner compiles a virtual-pointer fixture from the vendored MIT Wayland
protocol, then sends input only to a private headless compositor. It requires
`wayland-scanner`, a C compiler, `wayland-client`, labwc, D-Bus, wtype and grim.

Multi-output ownership/hotplug, window tracking, drag reordering, running-app
indicators and compositor background blur remain separate migration work. The
current dock launches pinned apps; it does not invent running-state indicators.

The build fingerprints both external CMake archives in a link flag so SwiftPM
relinks after C++ changes. The focused-field lifecycle patches also protect
cross-window focus bookkeeping; see `Patches/`.
