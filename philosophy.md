# Meridian — Product Philosophy

## Mission

Build **Meridian** into a world-class desktop environment: beautiful because it is coherent, fast because it is deliberate, and trustworthy because every control works. The user should feel that the entire desktop was designed by one team with one vision.

## Principles (ordered)

1. **Consistency is a feature.** The desktop, system shell, settings, and first-party apps must all express one visual and interaction language. Prefer improving the shared system over patching one screen.
2. **Dark is the identity, not an option.** Meridian is a strictly dark-only environment. Pure black is the foundation. Do not implement light mode or make the UI follow host light appearance.
3. **Color has boundaries.** Text must remain white or black, even on hover. Color is allowed only in prescribed accents such as solid icon tiles, hover backgrounds, and non-text indicators; it must communicate clear state or improve recognition.
4. **Minimum UI, maximum capability.** Every visible label, separator, panel, title, and button must earn its space. Prefer intuitive icons, proximity, and progressive disclosure; never remove discoverability or accessibility to appear minimal.
5. **Compact, not cramped.** Small uniform icons, restrained padding, and dense controls must remain readable and navigable by mouse and keyboard.
6. **One component, many contexts.** A toggle in Control Panel is the same toggle in Settings. A menu item in the launcher behaves like a menu item everywhere. Shared implementations are a core architecture requirement.
7. **Predictability over novelty.** Stable shortcuts, stable icon mappings, stable hover behavior, consistent placements, and reversible actions. Avoid random changes across renders, surprise animations, or mysterious controls.
8. **Actual desktop integration.** Don't simulate a working system by altering only a frontend state. Bind UI to verified OS capabilities, propagate errors clearly, and preserve device state accurately.
9. **Performance is part of design.** Startup speed, low input latency, smooth motion, modest memory use, and efficient background activity are first-class UX characteristics.
10. **Accessibility is not optional.** Icon-only presentation still needs accessible names, tooltips, focus visibility, keyboard operation, and sufficient contrast. Minimalism must not exclude users.
11. **Reliability before spectacle.** Favor stable window management, input, notifications, power management, and persistence over ornamental effects.
12. **Taste through restraint.** No overwhelming accent rainbow, gratuitous glass, massive controls, huge icons, or disconnected visual themes.

## Product decisions that are already made

- Light mode is removed, not kept as an unused alternative.
- Main backgrounds are pure `#000000`.
- Text colors are `#FFFFFF` and `#000000` only, including hover and disabled states (opacity may vary).
- Global hover background modes are `Auto`, `Green`, `Blue`, and `Grey`.
- Icon styling is macOS-inspired: small, uniform, with restrained solid blue/white/gray/green tile backgrounds.
- Control Panel is compact, icon-first, and has **no visible labels in the main toggle grid**.
- Shared UI components and design tokens are required desktop-wide.

## Decision framework for the coding agent

When choosing between two implementations, ask:

1. Does it preserve the non-negotiable design rules?
2. Does it make the whole desktop more consistent, or only this particular view?
3. Can it reuse or improve a shared component?
4. Is the interaction discoverable, accessible, and truthful about system state?
5. Does it reduce complexity for users without increasing hidden bugs?
6. Can it be tested, maintained, and extended in the existing codebase?

If a proposal looks impressive but introduces inconsistent UI or fake functionality, reject it.

## Agent working standards

- Inspect the current repository architecture and existing patterns before coding.
- Read `design.md`, `Apps.md`, and `features.md` when working on related surfaces.
- Identify shared tokens/components before changing per-app CSS or views.
- Deliver small, integrated improvements rather than detached prototypes.
- Avoid broad, speculative rewrites that break existing functionality; migrate incrementally with a clear end state.
- Do not fabricate app capabilities, routes, files, or APIs. Verify them in the repository.
- Add tests for behavior and visual states wherever practical.
- Report what was changed, what was verified, and what still needs work.
- Treat this documentation as a directive; where it conflicts with old styling, move toward this documented system.

## Definition of world class

World class does not mean maximal UI decoration. It means that launcher, dock, settings, control panel, applications, windows, transitions, and interactions feel like the same fast, polished, reliable product at every moment.
