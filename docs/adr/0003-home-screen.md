# ADR-0003: The home screen is the launcher (no taskbar, dock, or panel)

Status: superseded by ADR-0005

## Context

Traditional Linux desktops put a permanent taskbar or dock on screen, plus a
start menu, desktop files, and a status tray. We want a minimal
experience where the desktop itself is where you find and launch things.

## Decision

- The **home screen** shows only the wallpaper, a search bar, and a text
  list of installed applications (text only, see ADR-0004).
- There is **no persistent chrome**: no panel, dock, or tray. The home screen
  is the desktop when no windows cover it, and it is summoned over windows
  with Super or the top-left corner.
- **Window switching** uses Alt+Tab (labwc's switcher in M0, Meridian's
  overview in M1).
- **System controls** are in a summoned **Control Center** (Super+C,
  top-right corner).
- Search is a native **provider registry** (§5 of ARCHITECTURE.md), with
  applications as the first provider.

## Consequences

- Nothing on screen competes with apps. Status such as time and battery is
  one gesture away, not always visible. A future opt-in minimal status strip
  could be added if users need glanceable status.
- Discoverability relies on the corners and on onboarding. The first-run
  experience (M4) must teach Super, Super+C, and Alt+Tab.
- The home surface changes layer (background ↔ overlay). That needs
  layer-shell v2+, which every relevant compositor supports.
- Until M1 there's no visual indication of running apps on the home screen.
