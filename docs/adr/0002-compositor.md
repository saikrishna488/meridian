# ADR-0002: Use labwc as the initial compositor; protocol-only coupling

Status: accepted (M0)

## Context

Writing a production Wayland compositor is a multi-year effort. The shell
needs a compositor that supports a floating (stacking) window model and the
shell-facing protocols: layer-shell, foreign-toplevel, session-lock,
output-management, fractional-scale.

## Options

- **labwc**: wlroots, stacking, small, stable, XWayland, all needed protocols.
  Configured with XML. Has no animations of its own.
- **Wayfire**: wlroots, stacking, has animations, but a larger plugin surface
  and more of its own UI opinions.
- **KWin / Mutter**: full-featured, but they ship their own shell, and
  replacing that shell isn't supported (Mutter has no layer-shell).
- **sway / Hyprland / niri**: tiling or scrolling models. Not the default UX
  we want.
- **Own compositor on Smithay** (Rust): the long-term answer; too early now.

## Decision

Ship labwc as the M0–M4 session compositor (`session/labwc/`). The shell
couples to it **only via standard Wayland protocols** and fixed D-Bus actions
(`gapplication action org.meridian.Shell …`) bound to keys in labwc's config.

## Consequences

- No window open/close animations until our own compositor exists.
- The shell runs on any layer-shell compositor. CI and development can run
  nested.
- The Smithay compositor (M5+) can be developed in parallel and swapped in
  without changing the shell.
