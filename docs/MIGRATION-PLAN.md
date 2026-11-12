# Linux Swift / C++ migration plan

Target: SwiftCrossUI/GTK4 GUI, C++20 services with explicit ownership, a C ABI,
and the existing labwc Wayland session. Preserve behavior and capability checks.
Keep the current session installable until the replacement passes integration
checks; switching defaults is the final milestone.

## Current execution priority: backend first

Implement and verify C/C++ backend services before further Swift GUI work.
The existing Swift launcher remains a prototype. Backend order: protocol and
authorization foundation; configuration and application/window services; Linux
system controls; files/PTY/monitoring/notifications; authentication and installer.
Then resume the Swift desktop and surface integration milestones below.

The native library exposes generated schemas and capability policies for all
155 methods, typed validation, trusted surface clients, bounded request admission,
worker dispatch and cancellation. Implemented slices include atomic configuration,
fuzzy application search/launch, defaults/startup, basic files, staged transfers,
Trash browsing/restore, PipeWire audio controls and selected system services.
Pins, the toolbox host bridge and many other handlers remain pending. These
implementations still require full service/session qualification; registration
and passing isolated tests do not establish complete desktop parity.

## 1. Compile and validate the launcher (debug build verified)

- [x] Native app discovery/launch with immutable RAII catalog and bounded errors.
- [x] C ABI and isolated XDG catalog tests, including ASan/UBSan.
- [x] SwiftCrossUI launcher source and shared rounded card primitive.
- [x] Build with a real Linux Swift compiler; fix API/link errors.
- [x] Add repeatable checks and pin resolved dependencies.
- [x] Launch under nested Wayland; verify keyboard search, Return launch and Escape dismissal.
- [x] Verify refresh discovers an application added while the Swift UI is open.
- [ ] Verify app-launch failures through the Swift UI.
- [ ] Qualify optimized release builds and physical keyboard/accessibility behavior.

Acceptance: reproducible Swift/C++ build, no Rust/WebKit linkage in the new
executable, working keyboard navigation and app launching in a nested session.

## 2. Native Wayland surface host

- [ ] C++ layer-shell configuration for applications, top bar, panel and wallpaper.
- [x] Attach Swift launcher content before mapping; explicit main-thread lifetime rules.
- [ ] Monitor selection/hotplug, geometry, keyboard focus and teardown.
- [ ] Nested-session tests covering exclusive zones, focus and reopen.

Acceptance: Swift launcher is a real overlay; bars reserve correct work areas;
closing/reopening and display changes do not leak or crash.

## 3. Services and protocol

- [x] Inventory all 155 existing Request variants and their required capabilities into a parity checklist.
- [ ] Port typed request validation, authorization, rate limits and bounded async work.
- [ ] Port config persistence, app search/icons/pins and window tracking/activation.
- [ ] Port connectivity, audio, brightness, displays and power through existing Linux APIs.

Acceptance: each implemented service has equivalent request validation, capability
boundaries, errors and cancellation behavior; unavailable hardware is reported.

## 4. Swift desktop and settings

- [ ] Port desktop/wallpaper, dock, top bar and Control Center.
- [ ] Port Settings by category, including accessibility and recovery.
- [ ] Port notifications, Files, terminal/PTY, monitoring and screenshots.
- [ ] Shared appearance tokens; light/dark, keyboard/focus and reduced-motion review.

Acceptance: every existing UI workflow has an implemented replacement and a
recorded manual/integration result. No placeholder controls stand in for features.

## 5. Authentication and session safety

- [ ] Port compositor-enforced session locking, PAM, greetd and multi-monitor lock.
- [ ] Exercise failure paths: auth unavailable, UI crash, monitor hotplug and resume.

Acceptance: locked content never becomes accessible when auth/UI components fail.

## 6. Installer and switch-over

- [ ] Port installer orchestration and release verification to C++.
- [ ] Update dependencies, packaging, development scripts and distribution checks.
- [ ] Measure RSS/peak memory/idle CPU/launch latency against the existing build.
- [ ] Run full regression suite and nested-session workflow checks.
- [ ] Switch defaults and remove React/WebKit/Rust dependencies after parity passes.

Acceptance: verified install/update/repair/uninstall; all parity items pass;
measured resource results published alongside outstanding hardware qualification.

## Execution notes

2026-11-11: C++ catalog/C ABI tests and headless labwc overlay map/teardown/reopen
pass. Swift debug build compiled with project-local Fedora Swift 6.3.2/aarch64.
Virtual-keyboard search, refresh discovery, Return launch and Escape dismissal pass with fatal GTK
critical checks. Fixed native PIC linkage, Swift callback/state ownership and a
pinned dependency shutdown/focus bugs. Existing uncommitted user edits are preserved.
See SWIFT-CPP-MIGRATION.md for commands, compatibility patches and limits.

Backend continuation: added synchronous/asynchronous file transfers with progress,
cancellation, staged publication and held-source recovery; GIO Trash listing and
collision-safe restore; PipeWire device/stream controls with refreshed identity
checks and virtual outputs. Fourteen backend contracts pass in normal and
ASan/UBSan builds. Real cross-filesystem moves and collisions pass using private
fixtures in `/tmp` and `/dev/shm`. Audio is qualified with command fixtures;
live PipeWire and GVfs session qualification remain pending. See
[backend progress](BACKEND-IMPLEMENTATION-PLAN.md#progress-evidence).

## Native desktop prototype continuation

A Swift workspace now connects application launching, Files browsing/preview/
folder creation/default-app opening, persisted search preferences and system
information to the C++ backend. Optional session mode adds Swift wallpaper,
top-bar and dock layer surfaces with reserved work areas and workspace reopening.
`tools/dev-native-session.sh` starts an isolated native-only compositor session.
See [prototype instructions](NATIVE-PROTOTYPE.md). Full UI/service parity and
production authentication remain pending; no installed defaults are switched.
