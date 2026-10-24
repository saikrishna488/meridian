# Meridian Design Philosophy

This document guides future visual and interaction changes to Meridian. Keep it
near the project root so contributors and coding agents can find it before
changing the desktop.

## Product character

Meridian should feel like a calm, coherent Linux desktop: familiar enough to use
without instruction, polished without visual noise, and honest about what the
system can do. Prefer platform services and established protocols over custom
implementations of security-sensitive system behavior.

## Visual principles

- Use a restrained palette: near-black and warm white surfaces, quiet borders,
  and one clear accent for selection and focus.
- Prefer solid, legible surfaces for desktop chrome. Wallpaper may provide
  atmosphere; controls should not depend on gradients or wallpaper contrast.
- Use consistent rounded corners, spacing, typography, and icon sizing across
  surfaces. Edges and hover/focus states should feel smooth and deliberate.
- Keep shadows subtle and functional. Avoid glossy decoration that competes
  with content or reduces contrast.
- Preserve light and dark themes, reduced motion, high contrast, and keyboard
  focus visibility in every shared component.

## Interaction principles

- Make controls predictable and reversible. Confirm actions that can remove
  data, end a session, or change system state.
- Keep feedback close to the action, use plain language, and distinguish
  unavailable system capabilities from failed actions.
- Support keyboard use and visible focus; hover must not be the only signal.
- Closing a surface should release the work and resources it owns, including
  terminal processes and polling tasks.

## Architecture and security

- Reuse Linux services, desktop standards, and compositor protocols where they
  provide a better maintained or safer implementation.
- Never implement password handling or session authentication in UI code when
  an established PAM-aware system utility can own that responsibility.
- Keep privileged operations narrow, explicit, and separately confirmed.
- Treat the Rust host and typed protocol as the boundary for system access;
  validate requests and avoid shell interpolation of user-controlled values.
- Keep UI assets local and surfaces isolated. Avoid adding network dependencies
  to ordinary desktop rendering.

## Guidance for design changes

Before changing a shared component, inspect its use across all surfaces and
update shared tokens or components rather than introducing one-off styles.
Prefer small, reviewable changes that preserve existing workflows. Update the
relevant docs when behavior, installation, or system requirements change.
