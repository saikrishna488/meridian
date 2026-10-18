# ADR-0008: Recoverable desktop components

Status: accepted for the Alpha 0.2 engineering candidate; release qualification
is tracked in [Phase 1](../PHASE1.md).

Each surface gets its own WebKit context and ephemeral network session. A
renderer failure reloads that view with bounded exponential delay; a missing
handshake also triggers recovery. Commands sent during startup are coalesced and
menu/location state is restored. Hidden application windows remain cached, so
closing them does not repeatedly create WebKit contexts.

Isolation increases process count and memory overhead relative to the original
shared-context design. Measure the entire process tree during hardware soak
testing; old single-process footprint estimates are no longer applicable.

GTK owns ordinary window lifetime independently of Rust wrappers. Discarded
ordinary surfaces explicitly destroy their windows and cancel their retry
timers. Lock windows are an exception: gtk4-session-lock owns their unmap and
destruction. Meridian must not destroy or hide an active lock window itself.
A native GTK password form uses the same serialized PAM verification and
current-lock checks while a lock renderer recovers.

The session supervisor restarts only the shell with a bounded delay. labwc stays
independent, allowing unrelated application windows to survive a shell failure.
Per-compositor file locks complement GApplication uniqueness across buses.
A lost compositor/GTK display connection is a session failure, not something a
WebView reload or window-list reconnect can repair. An abandoned compositor lock
must remain fail-closed; recovery on each supported compositor needs explicit
hardware qualification.

Nested tests run on the OS main thread, treat GTK critical errors as failures,
and exercise renderer crashes, lock lifetime, terminal scrolling, repeated
application use, window-service reconnection and full shell-process replacement.
