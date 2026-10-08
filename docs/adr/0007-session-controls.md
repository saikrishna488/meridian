# ADR-0007: Session controls in the Options menu (Lock, Sign out, Sleep)

Status: accepted with implementation changes.

> Current implementation: session actions are in the Meridian menu, and the
> Session capability belongs to DesktopMenu, not Options. Restart and Shut Down
> are also available. The lock/PAM design below remains relevant; descriptions
> of the original Options rows are historical. See [current architecture](../ARCHITECTURE.md).

## Context

ADR-0005 shipped the Options menu with Wi-Fi, Bluetooth, brightness, and
volume, and explicitly deferred power controls: "Clock, battery, and power
controls aren't shown in M0 by request. They can return as bar items or
Options rows." Meridian has no way to lock the screen, end the session, or
suspend the machine without dropping to a shell. ADR-0002 already chose
labwc partly *because* it implements the Wayland protocols a shell needs,
including `ext-session-lock-v1` — so Lock has a real native path, not just
a shell-out.

## Decision

Three rows in the Options menu: **Lock**, **Sleep**, **Sign out**
(`shell/src/shell.rs`, `ui/src/surfaces/options/`). Lock also gets a
keybinding (Super+L, `session/labwc/rc.xml`), like Applications and
Options already have, via the same fixed `gapplication action
org.meridian.Shell lock-session` mechanism (ADR-0002).

- **Sleep** and **Sign out** are D-Bus calls to logind
  (`shell/src/power.rs`, shared with the greeter's existing
  restart/shut-down code):
  - Sleep → `Manager.Suspend(interactive: false)`.
  - Sign out → `Manager.GetSessionByPID(our pid)` then
    `Session.Terminate()` on the result. Terminating the session takes
    labwc and everything in it down with it, returning to the login
    screen, the same place logind leaves the greeter's own restart/shut
    down actions.
- **Lock** is a new full-screen surface (`SurfaceKind::Locker`,
  `ui/src/surfaces/locker/`), shown on **every** monitor via
  `ext-session-lock-v1` (the `gtk4-session-lock` crate, a sibling of
  `gtk4-layer-shell` built on the same C library, version ≥ 1.2.0). Unlike
  the Applications/Options menus (layer-shell overlays with exclusive zone
  0), a session lock is enforced by the *compositor*: once
  `gtk4_session_lock::Instance::lock()` succeeds, labwc stops rendering
  and routing input to every other surface until `unlock()` is called.
  Meridian doesn't have to hide the panel or reject input itself.
  - One lock surface per monitor (`Shell::add_lock_surface`,
    `Instance::connect_monitor`), all showing the same password prompt, so
    the user doesn't have to find a specific screen to unlock from.
  - The password is checked with PAM (the `pam-client` crate) against a
    new `meridian-lock` PAM service (`session/meridian-lock.pam`,
    installed to `/etc/pam.d/` by `tools/install.sh`), the same mechanism
    swaylock and i3lock use. This only ever checks the password of the
    user already running the session (never an arbitrary username), which
    is what lets an unprivileged `meridian-shell` do it: `pam_unix` goes
    through the setuid `unix_chkpwd` helper for that case.
  - The PAM check runs on `gio::spawn_blocking`'s thread pool, not the
    GTK main loop, since `pam-client`'s API is synchronous and a stacked
    PAM module (network directory, deliberate throttling after a wrong
    password) can be slow.
  - A new `Capability::Unlock`, held only by `SurfaceKind::Locker`, gates
    `locker.unlock`; a new `Capability::Session` on `SurfaceKind::Options`
    gates `session.lock` / `session.sign_out` / `session.sleep`.

## Consequences

- Locking needs `ext-session-lock-v1`. Compositors that implement
  layer-shell but not session-lock (checked with
  `gtk4_session_lock::is_supported()` at startup) can run the rest of the
  shell; Lock fails with a logged warning instead of blocking startup.
- No fingerprint or two-factor prompts on the lock screen either, same
  limitation as ADR-0006's login screen: a stacked PAM module that needs
  another prompt fails, since the conversation handler only ever answers
  with the one password.
- Sign out is abrupt by design (logind kills the session's cgroup): there
  is no "save your work" negotiation with running apps. The Options row
  requires clicking twice, the same confirmation the greeter's power
  buttons already use.
- Building needs `pam-devel` (`tools/dev-env.sh`, `docs/DEVELOPMENT.md`);
  running needs nothing new; `gtk4-layer-shell` ≥ 1.2.0 is already a
  runtime dependency and Fedora 44 ships 1.3.0.
