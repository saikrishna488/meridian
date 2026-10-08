# ADR-0005: Top bar with Applications and Options menus

Status: accepted with implementation changes.

> Partially superseded: the current shell has a 32-pixel menu bar, a separate dock, desktop shortcuts, a clock, and Control Center. The original single-bar layout below is retained as history.
> See [current architecture](../ARCHITECTURE.md).

## Context

The product direction changed from "the desktop is the launcher" to a
single top bar. The bar has an Applications menu, the running windows, and
an Options menu with the most-used system controls.

## Decision

- **Top bar** (layer-shell, top layer, reserves 36 px): "Applications", the
  running windows (text, focused one marked, click to focus), and "Options"
  at the far right.
- **Applications** is a dropdown with search and a text list of installed
  apps. It reuses the provider-based search and keyboard model.
- **Options** is a dropdown with Wi-Fi and Bluetooth switches and Brightness
  and Volume sliders (0–100%), backed by NetworkManager, BlueZ, logind
  (+ sysfs), and WirePlumber.
- **The desktop** is wallpaper only. No hot corners.
- Menus are overlay surfaces with exclusive zone 0, so they sit below the
  bar and the bar stays clickable.

## Consequences

- Running windows need `wlr-foreign-toplevel-management`, which labwc, sway,
  Hyprland, and Wayfire support. KWin and Mutter don't, and on them the
  window list is disabled with a log warning.
- Clock, battery, and power controls aren't shown in M0 by request. They
  can return as bar items or Options rows.
- Volume goes through the `wpctl` CLI (fixed argv) until M2's native
  PipeWire service. Volume changes made elsewhere show up the next time
  Options opens.
