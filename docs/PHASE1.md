# Meridian 0.2 stability qualification

> Nova and Vault have since been removed from the project. References below describe the earlier implementation.

> This is a qualification record for an earlier build. The current package
> delegates file browsing and terminal sessions to GNOME Files and Kitty; its
> embedded Finder/Terminal references below describe legacy source and prior
> integration coverage. Current feature claims are in the [README](../README.md).

Status: engineering candidate; **not yet qualified as Stable Alpha**. Do not tag
0.2 or declare Phase 1 complete until the hardware and endurance gates below have
recorded results. Automated tests are evidence for their particular fixtures,
not proof that real PAM, drivers, storage devices, or suspend work correctly.

## Automated coverage and implementation

| Area | Implemented and exercised | Remaining qualification |
| --- | --- | --- |
| Renderer failures | Separate WebKit contexts/network sessions; exponential reload delay capped at 30 seconds; 20-second handshake watchdog; coalesced startup events and menu/settings location restoration. Nested test repeatedly terminates launcher and Control Center renderers while the dock survives. | GPU-driver resets, renderer OOM under pressure, extended crash/recovery cycling. |
| Shell failures | Per-compositor advisory instance lock, including separate D-Bus sessions; bounded supervisor restart delay; labwc remains the session owner. Nested test SIGKILLs the real supervised shell and verifies a separate application survives and a replacement shell starts. | Hardware session, shell death while locked, and recovery from the compositor itself failing. |
| Outputs/window list | Discard removed-output surfaces, deduplicate lock surfaces, dismiss menus before moving bars, preserve ordinary windows, and reconnect an unavailable window-list service every five seconds. Window identifiers do not repeat after reconnect. | Physical disconnect/reconnect while unlocked, locked, and suspended; actual protocol-global withdrawal. A dead GTK display/compositor connection is not recoverable inside this process. |
| Resource lifetime | Destroy discarded GTK windows; remove Wayland fd watches; release notification registration/name ownership; clear terminal PTYs on hide/reload/shutdown. Nested test checks window count and terminal cleanup over 50 open/close cycles and records shell plus process-tree RSS. | 24-hour soak with aggregate memory, fd, process and responsiveness observations. Cached application windows intentionally remain while the shell runs. |
| Bridge | Exhaustive Rust capability match; native authorization tests and real WebKit denial checks; payload-free logs; non-echoing parser errors; 2 MiB requests; 4 KiB search; 256 requests/second per view; 16 outstanding async requests per view and 128 globally. Shared client handshake is idempotent. | Independent security review and fuzzing beyond the malformed/oversized regression corpus. Resource limits bound work but are not an OS-level sandbox for arbitrary native operations. |
| Resource scheme/navigation | Canonical root containment; encoded traversal and symlink-escape tests; regular-file/32 MiB limits; own surface's entry document only; permission requests denied. The sole navigation exception opens the fixed project URL externally only for a user gesture, at most once per second. | Independent race audit of resource roots writable by the same user. |
| Orbit transfers | Private staging directories; no-overwrite atomic finalization; file/directory sync before completion; bounded worker count; progress, cancellation and reconnect after page reload; symlink preservation; conflict, injected write-failure, space, cancellation, 2,048-file and cross-filesystem tests. Desktop moves use the same transfer engine; native drops run off the GTK thread. | Quotas, removable media unplug, real ENOSPC mid-write, I/O errors, permissions/ACLs, and sudden power loss on actual supported filesystems. |
| Authentication/lock | Native GTK password fallback retains the session-lock window role during WebKit failure. PAM attempts are serialized across outputs with failure backoff; obsolete lock surfaces cannot unlock a later lock. Two-output nested lock/recovery cycles and rejected oversized input are tested without calling real PAM. | Real successful/failed PAM, PAM failure policy, account expiry, greetd, and resume tests in disposable accounts/VMs. A PAM module that hangs can hold the single authentication worker; it cannot freeze the GTK event loop. |
| Login/power | Bounded greetd frames/conversation and input; login concurrency guard; 90-second login deadline; socket close on abandonment; interrupted/malformed socket tests; logind call deadlines; session-ending power actions reject active Orbit transfers. Existing compositor-confirmed lock-before-sleep/standby controls remain mandatory. | Real greetd cancellation/retry and logind/PolicyKit behavior. External power actions are controlled by the system, not Meridian's bridge guard. |
| Configuration | Bounded reads and atomic private writes for desktop, bookmarks and power; corrupted content is backed up before replacement. Regression checks corrupted and oversized files. | Remaining configuration stores and disk-failure testing during persistence. |
| Installation | Verified build manifest; file snapshots before installation; automatic file rollback on a failed installation; fixture tests for a clean file install and failure after partial replacement. | Clean VM installation, upgrade, reboot and display-manager rollback. Package-manager changes are outside file rollback; SIGKILL/power loss requires inspecting the saved installation snapshot. |
| Terminal | Selection-aware Ctrl+C, single Ctrl+V/Ctrl+Shift+V insertion, no shell footer, correct fit measurement, stable scrollback controls. Actual WebKit wheel scrolling, return-to-bottom and unclipped terminal grid are checked in nested Wayland. | Mouse wheel/touchpad on physical devices and applications using alternate-screen mouse tracking. |

## Bridge capability audit

Every `Request` variant is assigned in `Request::required_capability`; its
exhaustive match makes an unclassified new variant a compile error. Only
`shell.hello` and `desktop.get` are intentionally public to first-party surfaces.
Authorization occurs before native dispatch. While the compositor confirms a
lock, the shell dispatcher admits only the handshake, desktop appearance/state
read, and unlock. Unlock additionally checks membership in the current set of
lock surfaces before and after PAM finishes.

| Capability | Requests | Surfaces |
| --- | --- | --- |
| Terminal | `terminal.start`, `start_at`, `read`, `write`, `resize`, `close`, `copy`, `paste` | Terminal only |
| Files | All `files.*`, plus `terminal.open_at` | Orbit only |
| DesktopFiles | `desktop.add_file`, `choose_files`, `remove`, `open` | Wallpaper and Orbit; dock pinning no longer grants desktop file removal |
| SystemAdministration | User, driver and device inventory/actions | Settings only; sensitive actions still use AccountsService/PolicyKit or authenticated, fixed helpers |
| Login / Unlock | `greeter.*` / `locker.unlock` | Greeter / current lock screens respectively |
| Settings | Connectivity, displays, sound, idle, system information and release checks | Settings and Control Center |
| Startup | Startup inventory, enable/disable and add | Settings only |
| Appearance | Font, appearance and wallpaper changes/picker | Settings and Control Center |
| PowerStatus | Battery/AC read | Settings, top bar and Control Center |
| Session | Lock, sign out, sleep, restart, shutdown | Desktop menus and Control Center |
| Dock | Pins, layout, Trash opener, desktop application shortcuts | Dock, launcher, Orbit and wallpaper |
| Search | Installed-app search and activation | Dock, top bar, launcher and Orbit |
| Windows | List, activate and graceful close | Dock, top bar, launcher and desktop menus |
| Popovers | Menu controls and built-in window openers | Wallpaper, dock, top bar, launcher, desktop menus, notifications, Orbit and Control Center |
| AppManagement | App inventory/store and reviewed uninstall | Dock, launcher, Orbit and Nova |
| Monitor / Calendar | Process metrics / calendar persistence | Pulse / Calendar respectively |
| Notifications | Read/dismiss/action notification history | Top bar and Notification Center |

Privilege boundaries: the shell remains the logged-in user. Account operations
use AccountsService's authenticated D-Bus API; driver/package/device actions use
fixed argument arrays and revalidated inventory/targets. Password hashing uses
stdin, never command arguments. Lock PAM targets the running account and performs
both authentication and account management. The greeter sends credentials only
to greetd's Unix socket. No root helper, arbitrary shell-command bridge endpoint,
or blanket JavaScript clipboard/system permission was added.

## Transfer failure and crash recovery

Transfers never merge into or overwrite an existing destination. A same-device
move is a no-overwrite rename. Copies and cross-device moves publish a complete
staged tree using `renameat2(RENAME_NOREPLACE)` on the destination filesystem.
Unsupported filesystem semantics fail closed. Cancellation is checked while
walking and for each 64 KiB file chunk; once atomic publication has begun,
completion wins over a late cancellation.

Cross-device moves hold the original under a private sibling
`.meridian-transfer-XXXXXX/data` while copying. Its `recovery.json` records the
original path before the source rename. Errors attempt a no-overwrite restore;
if that path has been occupied, the error identifies the recovery directory.
Unwinding preserves held source data. A crash or power failure can leave these
hidden recovery folders. **Do not delete a held-move folder until its `data`
has been restored or verified against a complete destination.** Copy-stage
folders can contain incomplete copies; their source remains untouched. Recovery
is deliberately manual rather than trusting a writable manifest to delete or
replace a user's files automatically.

The safety budget is two asynchronous transfers, 200,000 entries and 128 folder
levels per transfer. Completed job state is bounded and retained for page
reconnection. File contents are checked for size/mtime changes during copying;
this is not a snapshot of files concurrently modified by other processes. Avoid
editing active transfer sources. ACLs, xattrs, hard-link identity and sparse
allocation are not currently preserved; symlinks and ordinary permission bits
are preserved, without copying setuid/setgid bits. Do not use this alpha as a
metadata-preserving backup tool.

## Reproducible checks

```sh
tools/check.sh
# Needs labwc, dbus-run-session, a compiler, GTK development files and built UI:
tools/tests/wayland.sh
# Optional endurance run in the same disposable desktop:
MERIDIAN_SOAK_SECONDS=86400 tools/tests/wayland.sh
```

The nested test uses separate Wayland sockets, D-Bus, XDG configuration/data and
a disposable GTK application. It does not use the real login account's password
or call host suspend/shutdown. The headless fixture uses software rendering; physical GPU qualification remains separate. GTK/WebKit run on the OS main thread in the
feature-gated `meridian-smoke` executable; it is excluded from normal builds.
CI requires the nested test after formatting, Clippy, Rust, generated protocol,
TypeScript, UI build and interaction tests. Its root-only WebKit sandbox override
is confined to the disposable CI container/test script, not installed sessions.

Record build commit, package versions, hardware/driver, filesystem and test
commands with each result. Summed RSS includes shared pages and is an observation,
not a physical-memory measurement or proof of the absence of leaks.

### Local engineering result, 2026-10-18

Working tree based on `e2336ae`, before a release commit/tag. GTK 4.22.4,
WebKitGTK 2.52.6 and gtk4-layer-shell 1.3.0. Formatting, Clippy with all features,
195 Rust tests, TypeScript/build and 85 UI tests passed. Four Rust tests remain
ignored in the ordinary unit invocation: the nested scenario is run separately
by the smoke executable, while the image-loader and real GVfs fixtures require
their stated services. Release-manifest, screenshot, power, supervisor and
installation-transaction script regressions passed.

The strict nested run used two headless outputs and fatal GTK critical errors.
It passed repeated launcher/Control Center renderer termination, three lock
cycles with lock-renderer termination and native fallback, native bridge denial
checks, window-service reconnection, real terminal wheel/fit checks, 50 application
open/close cycles, and a further 60-second soak. Window count remained 11;
terminal sessions were cleared. Shell RSS changed from 324,384 to 313,844 KiB;
summed process-tree RSS changed from 2,748,356 to 2,732,100 KiB. These short-run
observations do not replace endurance qualification. A final real shell-process
SIGKILL/restart left the independent GTK application alive. No GTK critical
errors occurred in this run. Expected renderer-termination and disconnected-bus
warnings during deliberate crash/teardown are not hardware qualification.

## Required real-hardware/VM matrix before Stable Alpha

All rows below are **pending** until an operator records actual results. Use
fixtures and disposable accounts; do not repeatedly submit wrong passwords to a
production account subject to lockout policy.

| Platform/configuration | Required exercises | Evidence |
| --- | --- | --- |
| Debian testing VM, fresh install | Build/install, reboot to Meridian, normal logout, uninstall, display-manager restoration, failed upgrade rollback | Install journal, session logs, before/after installed file manifest |
| Intel or AMD integrated graphics, one display | 24-hour session, 100+ built-in opens/closes, browser/app crashes, shell restart, wheel and touchpad scroll | Shell + renderer memory/fd/process samples, responsiveness notes |
| Proprietary NVIDIA where supported | Same baseline, renderer crash and GPU pressure | Driver/version, recovery logs, visual checks |
| Laptop, internal + external outputs | Lock, plug/unplug each output, change primary, lid close, suspend/resume repeated 20 times | Compositor lock confirmation, output list, no exposed desktop while locked |
| Multi-monitor mixed DPI/refresh | Launcher/dock/Control Center recovery, disconnect output while a menu/app is open | Screenshots and logs; ordinary app windows remain usable |
| Real PAM and greetd, disposable account | Correct/incorrect passwords, throttling, account policy, cancel/interrupted login, greeter renderer loss | Authentication journal without secrets, ability to retry/unlock |
| ext4 and another supported filesystem + USB | Cross-device copies/moves, occupied targets, permissions, quotas/full disk, unplug during transfer, Trash conflict/restore | Fixture checksums, recovery-folder inventory, no overwritten originals |
| Missing/failed services | NetworkManager, BlueZ, PipeWire/WirePlumber, logind/AccountsService/PolicyKit availability and restart | Actionable UI errors and continued desktop responsiveness |
| Power loss/crash while moving or installing | Recover held sources and inspect installation snapshots in a disposable VM | Verified originals/destinations, successful boot/session rollback |

Release gate: all five user-defined completion criteria require recorded passing
results, including the physical authentication/resume and extended-session rows.
Automated coverage alone does not authorize the Stable Alpha designation.
