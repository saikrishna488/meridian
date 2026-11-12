# C/C++ backend implementation plan

Swift GUI development is deferred until backend parity and integration checks
pass. The Rust implementation is the behavior reference, not a runtime bridge.
Do not claim a port complete merely because its method is registered.

## Structure

| Directory | Responsibility |
| --- | --- |
| `native/include/` | Public C interfaces; owned handles and explicit lifetimes |
| `native/src/core/` | JSON ownership, errors, bounded I/O, atomic configuration |
| `native/src/protocol/` | Generated schemas/policies, validation, trusted clients, dispatch |
| `native/src/services/` | App, file, settings, terminal and other domain logic |
| `native/src/platform/` | Linux/Wayland host integration |
| `native/tests/` | Isolated contract and integration tests |
| `tools/migration-inventory.py` | Generate protocol metadata and detect Rust/native drift |

## Milestones and acceptance criteria

1. **Protocol and execution foundation — in progress**
   - Correct capability extraction and trusted surface permissions.
   - Validate every request shape, unknown fields, enums, numeric limits and size.
   - Bounded request admission and asynchronous execution with cancellation.
   - C ABI owns replies; exceptions and secrets never escape into diagnostics.
   - Verify permission boundaries, malformed input, rate limits and shutdown.
2. **Durable stores and applications — pending**
   - Private atomic writes, bounded reads, corrupt backups, future-version safety.
   - Port app metadata, fuzzy ranking, host bridge, pins/defaults/startup.
   - Verify isolated XDG fixtures, restart persistence and launch failures.
3. **Files and terminal — pending**
   - Files, bookmarks, previews, trash, transfers with cancellation/progress.
   - Owned PTYs, bounded output, resize, process supervision and cleanup.
   - Verify symlinks, collisions, traversal, failed/cancelled I/O and child exit.
4. **Linux settings and monitoring — pending**
   - Connectivity, audio, power, brightness, time, input, displays and storage.
   - Process/resource metrics, capability reporting, accounts and drivers.
   - Use Linux service APIs and existing privilege boundaries; no shell interpolation.
   - Fake-service contract tests plus recorded hardware/session qualification.
5. **Desktop state, windows and notifications — pending**
   - Desktop/dock/workflow/calendar state, window tracking and activation.
   - Notifications, screenshots and typed host actions/events independent of GUI.
   - Test persistence, notification ownership, Wayland reconnect/hotplug and cleanup.
6. **Authentication and installer — pending**
   - PAM, greetd and compositor-enforced lock lifecycle, including failure paths.
   - C++ install/update/repair/uninstall with transaction and release verification.
   - Never simulate authentication success or replace a lock with a normal window.
7. **Backend qualification — pending**
   - Verify all 155 requests against the Rust behavior and error/capability contract.
   - Run sanitizer, regression, release and nested Wayland checks.
   - Publish resource measurements and outstanding hardware requirements.
   - Resume Swift GUI work only after backend acceptance; switch installed defaults
     only after full desktop parity.

## Progress evidence

- Protocol foundation is implemented: generated schemas and policies (including
  block-bodied Rust capability match arms), trusted surface grants, typed
  validation, bounded admission/workers, cancellation and C ABI reply ownership.
- Existing services include atomic stores, calendar/bookmarks/search/idle state,
  fuzzy application search/launch, icon metadata, defaults and startup management;
  basic file operations; system information, battery, printers and time controls.
- Added transfer handlers: `files.transfer`, `files.transfer_start`,
  `files.transfer_status`, `files.transfer_cancel`, `files.transfers`. Copies
  stage before atomic no-replace publication. Cross-device moves retain a source
  recovery manifest until publication and destination durability succeed.
  Backend teardown cancels and joins owned transfer workers.
- Added `files.trash_list` and `files.trash_restore`: bounded GIO listing,
  exact listed URI matching, original-path validation and no-overwrite restore.
- Added `settings.audio` and `settings.audio_action`: PipeWire inventory,
  defaults, device/stream volume and mute, and owned virtual output management.
  Transient IDs are checked against current names before applying actions.
- Verification: `bash tools/check-native.sh --backend-only` and the same command
  with `--sanitize`: 14/14 contracts pass, plus 3 inventory regression tests.
  Sanitizer execution needs a non-traced process: LeakSanitizer cannot run inside
  the current sandbox tracer. No leak checks were disabled for the passing run.
- `bash tools/check-native.sh --wayland` passes backend contracts and the nested
  compositor overlay configuration/map/teardown/reopen check. The standalone
  surface CTest skips outside a running Wayland display; the nested test supplies
  its own compositor and exercises the surface afterward.
- Opt-in cross-filesystem verification:
  `target/native-backend/transfers-test /dev/shm`; uses private fixture directories
  and verifies moves in both directions and preservation on destination collision.
- Outstanding qualification: injected storage/crash/restore failures, live GVfs
  Trash round trips and PipeWire/physical audio devices. Terminal, monitoring,
  connectivity, display/input controls, windows/notifications, auth and installer
  still need implementation or completion. Swift desktop integration remains pending.
- Method-level parity remains tracked in `MIGRATION-PARITY.md`; pending rows are
  intentional until both implementation and required checks are complete.
