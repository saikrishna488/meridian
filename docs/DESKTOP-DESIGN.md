# Desktop design refinement

The desktop now uses a shared neutral palette, consistent selection states,
rounded controls, and restrained blue accents. Light and dark appearances cover
the launcher, taskbar, Control Center, notification cards, Orbit navigation, and
Settings. The taskbar keeps its existing 56px compositor reservation.

Applications has grid and list views. List view includes application descriptions;
both views keep search, keyboard selection, drag and drop, and application actions.
The view preference is stored locally when WebKit storage is available. A footer
makes navigation and context actions discoverable.

Settings Home includes the current wallpaper, working appearance and wallpaper
links, and everyday preferences. Every settings page shows its existing category
description. The lock screen identifies the locked session and keeps password
entry legible against every wallpaper.

## Lock and power

Control Center now provides Lock screen, Sleep, Restart, Shut Down, and Sign out.
Restart, shutdown, and sign out require a second click within three seconds.
Power actions still use the existing native capability checks and logind calls.
The Options surface now holds the Session capability alongside DesktopMenu;
application search and the locker do not receive that capability.

Sleep starts the native session lock and waits up to four seconds for compositor
confirmation. It refuses to suspend if the lock cannot be confirmed. Authentication
still uses the existing PAM service; no password handling has moved into scripts.

The labwc autostart starts `swayidle`, with these defaults:

| Inactivity | Action |
| --- | --- |
| 5 minutes | Lock the session |
| 5 minutes, 30 seconds | Confirm the lock, then put all displays in standby |
| Input after standby | Restore display power; the password is still required |
| Before system sleep | Request and wait for the native lock |
| After system resume | Restore display power |

`wlopm` changes display power without disabling outputs or rearranging windows,
following [labwc's integration guidance](https://labwc.github.io/integration.html#7-session-lock).
`swayidle -w` waits for the lock helper during sleep preparation, following the
[upstream swayidle documentation](https://github.com/swaywm/swayidle/blob/master/swayidle.1.scd).
External suspend ultimately follows logind's inhibitor policy and time limits.
The explicit Meridian Sleep action refuses to proceed without a confirmed lock.

The lock helper waits for the boolean state exported by the `lock-session`
GApplication action. Only the compositor's locked signal sets it to true. A failed
lock, unavailable shell, or timeout prevents the idle display-off command.

The runtime package list now includes `swayidle` and `wlopm`. Existing installations
need a rebuilt release, installation, and a new login session to run the idle
manager. There is no power timeout editor yet; defaults live in
`session/labwc/idle`. Browser previews cannot exercise hardware power management.

## Verification

- UI build, type checking, and interaction tests cover list-view navigation,
  saved view preference, power confirmation, error recovery, and icon fallback.
- Protocol tests verify session controls are restricted to desktop controls.
- `bash tools/tests/power.sh` checks the swaylock hand-off, lock refusal when
  swaylock is missing or fails, standby/wake commands, and lock-before-blank
  behavior with isolated command fixtures (a swaylock mock stands in for the
  real lock screen, so the developer's session is never locked).
- Rust workspace tests, formatting, and shell Clippy checks validate native changes.
- Visual previews use the actual WebKit engine with deterministic sample data.
  Physical standby, PAM unlock, and suspend/resume need verification in the
  installed session; automated checks do not operate the current desktop.
