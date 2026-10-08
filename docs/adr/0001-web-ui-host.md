# ADR-0001: Web UI rendered by WebKitGTK 6 in a thin Rust host

Status: accepted with implementation changes.

> Implementation update: the WebKitGTK/Rust choice remains current. GTK also supplies native window controls and dialogs. Terminal disables WebKit hardware acceleration. The original footprint estimate and alternative-engine comparisons below are historical, not measured guarantees or a current compatibility survey.
> See [current architecture](../ARCHITECTURE.md).

## Context

We want to write the shell UI in HTML/CSS/TypeScript, without Electron, and
without using GTK or Qt as our UI toolkit. The host has to:

1. create Wayland **layer-shell** surfaces (panel, dock, background, overlays);
2. embed a mature, GPU-accelerated, accessible web engine;
3. control exactly what web content can do.

## Options considered

| Option | Layer-shell | Maturity | Footprint | Notes |
|---|---|---|---|---|
| **WebKitGTK 6 + gtk4-layer-shell** | ✅ (gtk4-layer-shell) | Production (GNOME Web, Tauri) | One shared web process | GTK only as the window container |
| WPE WebKit 2.x, WPE Platform API | ❌ (no layer-shell in its Wayland platform yet) | Embedded/TV focused, desktop platform still young | Smallest | Same engine; good migration target |
| CEF / Chromium | via custom Ozone work | Production | ~100+ MB per process tree | Heavy; Electron in all but name |
| Servo | needs custom embedding | Not production-ready for this | Small | Promising long-term |
| Native Rust UI (iced, Slint) instead of web | ✅ | Good | Smallest | Rejects the web-UI investigation |

## Decision

Use **WebKitGTK 6** (`webkitgtk-6.0`) with **gtk4-layer-shell**, through the
gtk-rs Rust bindings (`gtk4`, `webkit6`, `gtk4-layer-shell` crates).

GTK is used *only* for: the main loop, `GtkWindow` as the Wayland surface
container, and `GdkMonitor` enumeration. Every pixel of
UI is drawn by web content.

This is the "strong technical reason" exception to *avoid GTK*: WebKitGTK is
the only mature embeddable web engine on Linux that can live inside a
layer-shell surface today.

## Consequences

- Web content gets WebKit's multi-process architecture and bubblewrap
  sandbox for free.
- Accessibility: WebKitGTK bridges the DOM accessibility tree to AT-SPI.
- All GTK/WebKit-specific code stays in `shell/src/surface.rs` and
  `shell/src/bridge.rs`. The UI and protocol don't depend on WebKitGTK, so
  moving to WPE Platform (once it supports layer-shell) or Servo is a host
  rewrite, not a UI rewrite.
- Startup cost: one web process (~60–90 MB RSS shared across all surfaces).
  We'll measure it per milestone (`tools/measure.sh`, planned).
