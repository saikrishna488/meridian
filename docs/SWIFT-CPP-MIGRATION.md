# Swift / C++ migration

The requested target is Linux: SwiftCrossUI with its GTK4 backend for the GUI,
C++20 for native services, and a narrow C ABI between them. SwiftCrossUI is an
open-source SwiftUI-style framework, not Apple's SwiftUI. It is pinned to commit
`931d67ecc7c4f2551b64d68a709c60521cde0a7c` (MIT license):
https://github.com/moreSwift/swift-cross-ui

## Desktop prototype

See [the runnable desktop prototype](NATIVE-PROTOTYPE.md) for the native workspace,
Files, persisted preferences, system information, and optional wallpaper/top-bar/
dock session surfaces. This extends the launcher slice described below.

## Implemented first slice

- `native/`: C++ GIO application discovery/launch and a GTK/layer-shell
  Applications overlay configured before Swift content maps. RAII owns GObject
  references; an opaque catalog owns an immutable snapshot. Borrowed strings
  remain valid until catalog destruction. Launch accepts only catalogued IDs,
  uses GIO Exec expansion, and never invokes a command shell.
- `swift-ui/`: searchable application launcher with refresh, launch errors,
  native buttons, and reusable rounded cards with light/dark appearance. No React, WebKit or Rust
  dependency in this new executable. Swift owns copied display strings and
  releases the C++ catalog in `deinit`.
- `tools/build-native.sh`: builds and tests C++, then builds the Swift executable.

Build on Linux with CMake 3.22+, C++20, pkg-config, GIO Unix 2.78+, GTK 4.18+, gtk4-layer-shell 1.2+ and
Swift 6+ (including SwiftPM):

```sh
bash tools/build-native.sh
./target/swift-ui/release/meridian-native-applications
# Full-output Wayland overlay (Escape dismisses it):
./target/swift-ui/release/meridian-native-applications --overlay
```

The framework and transitive Swift packages need network access on first build.
The C++ backend is independently testable without Swift:

```sh
bash tools/check-native.sh
bash tools/check-native.sh --wayland
# Also exercise a compiled launcher using an isolated virtual keyboard:
python3 tools/tests/native-wayland.py \
  --launcher target/swift-ui/release/meridian-native-applications
# Optional: --wtype /path/to/wtype --screenshot target/native-launcher.png
```

## Current limits and remaining work

Development now prioritizes C/C++ backend implementation and verification before
further Swift GUI work. `meridian_backend.h` exposes trusted surface clients,
typed request validation, bounded workers and cancellable tasks. Service handlers
cover selected stores, applications/startup, basic files, staged transfers, Trash,
audio and system controls; unported methods return an explicit unavailable error.
`meridian_policy.h` exposes generated registry/capability metadata; registration
alone does not establish handler parity. `meridian.h` exposes bounded-result,
case-folded app search and serialized GIcon metadata without a GUI dependency.

This is an opt-in migration slice, not a completed desktop replacement. The
installed session, installer and default build continue using Rust/React.
A debug build was compiled with Fedora Swift 6.3.2 on aarch64, using a
project-local toolchain. C++ tests exercise isolated XDG entries, launch
filtering, error bounds, a successful launch, and C ABI compatibility. The
headless two-output labwc test verifies overlay mapping, teardown and reopen.
The Swift launcher has passed virtual-keyboard search, refresh discovery,
Return-to-launch and Escape dismissal under fatal GTK critical checks. A screenshot was inspected.
Release optimization, physical input, accessibility and hardware qualification
remain pending.

The launcher supports both a regular window and an opt-in `--overlay` mode.
It does not replace the installed Applications surface yet. It lacks the
existing toolbox host-app bridge, fuzzy keyword search, icons, pins, and window
activation behavior.

## Dependency compatibility

`tools/build-native.sh` resolves the committed Swift lockfile and applies the
small reviewed patches in `swift-ui/Patches`. The application lifecycle patch
prevents a menu update after GApplication unregisters during shutdown; without
it Escape trips GTK's registered-application assertion. The focus lifecycle
patch prevents an unfocused field from clearing another control's keyboard
focus during a state update. Patch application is idempotent and fails when
the dependency no longer matches.

For Swift 6.3 with GLib 2.88+, the script applies the compiler's suggested
`-validate-tbd-against-ir=none` workaround for the Gtk backend's GDBus constants.
Other compiler/library combinations do not receive this flag. This disables
that compile-time symbol validator, not application validation or runtime checks.

Use `MERIDIAN_BUILD_CONFIGURATION=debug` for a debug build and
`MERIDIAN_BUILD_JOBS=2` to bound concurrent compiler work (the default).
Additional arguments are forwarded to `swift build`.

On this development machine the toolchain is extracted under `target/toolchains`
because system package installation needs a sudo password. Its local runtime
library directory must be on the loader path when invoking its compiler and
compiler plug-ins. This toolchain is a development artifact, not distributed
with MarsOS.

## Remaining ports

See [the staged plan](MIGRATION-PLAN.md) and
[all 155 service requests](MIGRATION-PARITY.md). Before changing the installed session:

1. C++ surface host: GTK/layer-shell lifetimes, monitor hotplug, focus, geometry,
   compositor integration and SwiftCrossUI embedding.
2. Service/protocol parity: explicit capability checks and typed requests for
   system controls, asynchronous operations, cancellation and bounded work.
3. Desktop, dock, top bar, Control Center and Settings with shared Swift widgets.
4. Files, terminal PTY, notifications, monitoring and screenshot tools.
5. Lock/greeter with compositor-enforced session locking, PAM and greetd;
   authentication must never fall back to a regular window.
6. C++ installer, distribution dependencies, release manifests, packaging and
   nested Wayland integration checks. Switch defaults only after parity passes.

C++ provides explicit native allocation and lifetime control; Swift GUI objects
still use ARC. RAII and owned containers are the default, with buffers and their
capacities explicit at the C boundary. Rewriting in C++ alone is not evidence of
lower memory consumption: compare steady-state RSS, peak RSS, launch latency and
idle CPU in equivalent nested sessions before claiming improvements.
