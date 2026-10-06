# ADR-0006: Login screen on greetd

Status: accepted (M0)

## Context

Fedora 44 KDE ships Plasma Login Manager, which has no theming API, so a
Meridian login screen can't be a skin on top of it. The login screen
handles passwords, so it must be simple and auditable, and it must not run
as root.

## Decision

- **greetd** is the display manager. It runs PAM, owns the VT, and starts
  the chosen session. It is small and packaged in Fedora, with an SELinux
  policy (`greetd-selinux`).
- **`meridian-greeter`** is the login screen: the same locked-down WebView
  host as the shell, with one full-screen surface whose only capability is
  `Login`. It runs as the unprivileged `greetd` user inside a minimal labwc
  with no keybindings or menus.
- The greeter relays the username and password to greetd over its socket
  (`shell/src/greetd.rs`). It never stores or logs the password: the
  request's `Debug` output redacts it, and a test checks that.
- Sessions come from `wayland-sessions` desktop files, so Plasma remains
  selectable. The last user and session are remembered (in the greeter
  user's cache), never the password.
- Restart and shut down go to logind (`PowerOff`/`Reboot`, non-interactive),
  with a click-twice confirmation.

## Rollout safety

`tools/install.sh` adds the Meridian *session* to the existing login screen
without touching it. Switching login screens is a separate, explicit step:
- `--try-login-screen` runs the greeter on VT 7 alongside the current one.
- `--enable-login-screen` switches on next boot.
- `--disable-login-screen` reverts.

Text-console recovery steps are printed when switching.

## Consequences

- No fingerprint or two-factor prompts yet: a second secret prompt fails
  with a clear message.
- No user list or avatars (text-only; the username is typed and remembered).
- Multi-monitor: the greeter shows on one output.
