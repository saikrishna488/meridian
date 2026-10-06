# ADR-0004: Text-only shell UI (no icons)

Status: accepted (M0). Supersedes the "icon-first" home-screen idea.

## Context

Shell UIs usually depend on application icons and symbolic glyphs. On Linux,
app icons come from whatever icon themes and desktop entries are installed,
and their quality, style, and resolution vary. That works against visual
consistency, one of our top priorities. The product direction is a calm,
typographic desktop.

## Decision

The shell UI uses **no icons**:

- **Home:** a text list. Each row has the app name (primary) and its generic
  name or description (muted).
- **Control Center:** labeled text rows. Battery has a plain level bar,
  which is a data readout, not an icon.
- **Window switcher (labwc, M0):** classic style with only app name and
  window-title fields.

Hierarchy comes from the type scale, weight, and contrast in
`ui/src/lib/tokens.css`.

## Consequences

- Visual consistency doesn't depend on third-party icon themes.
- Every element is self-describing text, which helps screen readers.
- Less code and attack surface: no icon-theme lookup and no
  `meridian://icon/` endpoint. The protocol carries no icon fields.
- Rows are taller than icons per item. Long lists rely on search, which is
  the primary path anyway.
- Revisiting icons later would be additive: an optional field in
  `SearchItem` plus a host endpoint.
