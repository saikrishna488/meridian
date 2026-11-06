# Desktop workflows

Source added 2026-11-04. This work has static source review only. No build,
automated tests, Node verification, desktop session or hardware checks were run.
Existing release-qualification gates remain open.

## Ownership and persistence

| Area | Implementation and limits |
| --- | --- |
| Recovery | Settings → System → Recovery lists instantiated panels, menus, wallpaper and Pulse. Reloads preserve independent app windows. The last 30 recovery/request failures are held in memory without raw request values. Settings' existing Reload page remains the path for its own UI. |
| Memory | A 30-second housekeeping timer drops eligible surfaces after five minutes hidden. Native pending requests, transfers and locking suspend reclamation. Pulse view/filter/sort state is held in the shell, not its ephemeral WebView. Settings, Files, Terminal and Control Center remain allocated. No measured RAM reduction is claimed. |
| Shortcuts | Nine managed shortcuts are stored in `meridian/workflows.json`. The session launcher derives `rc.xml` under the user runtime directory from the selected base configuration, linking its menu/autostart/idle/authentication helpers. Saves replace only known key attributes and request labwc reconfigure. Key names/modifier order are checked, along with explicit bindings and common defaults; arbitrary external compositor bindings cannot be inventoried. A failed reload reports that sign-in is required. |
| Displays | `meridian/display-profiles.json` stores at most 16 confirmed combinations keyed by connector, description and EDID when available. Exact matches restore after two-second hotplug debounce or resume. Existing confirmation/rollback and at-least-one-output safeguards apply. Unknown combinations retain compositor defaults. Legacy `displays.json` is considered only for matching connector sets when no profile store exists. Connector renaming, missing EDIDs and identical devices can prevent identification. |
| Sound | PipeWire output nodes and playback streams are discovered on demand. Writes validate the transient ID and node name before invoking fixed wpctl commands. Stream level/mute require reported PipeWire properties. One application can expose several streams. A device-specific application choice can override the default output. |
| Notifications | One native toast shows the newest arrival; history contains all retained arrivals. Toasts show up to three actions, with remaining actions in history. Toast expiry is at most eight seconds; sender-requested notification expiry can close actions sooner. Locking, Do Not Disturb and muted-app rules suppress toasts. The existing 100-item in-memory history remains. Per-app rules use self-reported names, not verified identities. |
| Screenshots | grim saves the original; slurp handles region selection. A native GTK preview offers numeric crop, red pen, annotation undo, clipboard copy and PNG export. Edits are memory-only until exported. Maximum 32 megapixels, 100 strokes and 4,096 points per stroke. Pen input requires a pointer; cropping, undo, copying and saving have keyboard controls. |
| Comfort | Text scaling (100–150%) uses WebKit zoom on Settings, Files and Pulse. Fixed compositor panel geometry stays unchanged. High-contrast controls and reduced motion apply to web surfaces, not native compositor animation or third-party apps. |
| Setup | Settings opens once for the new checklist outside development sessions. Completion is explicitly user-marked, not inferred from visiting a page. Dismiss/reopen from Home. Keyboard layout remains unsupported where the backend cannot verify it. |
| Undo | Up to 20 desktop layout or dock pin changes per shell session, shared across their surfaces. Restores save before consuming history. No file-operation, uninstall or account-operation undo is implied. |

All preferences use user-owned files. The workflow bridge is limited to Settings'
Rendering capability; notification rules use the existing Notifications capability;
Pulse view state uses Monitor. No arbitrary command execution endpoint is added.

## Manual review after installation

These checks are **pending**, not recorded successes. Rebuild and install only
when authorized for this machine; this task did not start any build or test process.

1. Start a fresh Meridian session. Review the setup checklist, dismiss it and reopen it from Home.
2. With keyboard only, navigate launcher → settings search → a control → dropdown → confirmation → back. Check visible focus, Escape/Tab behavior, focus return and announcements with an installed screen reader. Repeat at 150% and in both themes. Verify native screenshot controls separately. Screen-reader compatibility is unqualified.
3. Save/reset shortcuts and check explicit/default conflicts, reordered modifiers, invalid key names and session restart persistence. Exercise login with a damaged override file and verify that base shortcuts and idle locking still work.
4. Open Pulse, change its view/filter/sort, close for five minutes and reopen. Confirm state restoration and inspect memory manually. Confirm active file transfers and pending operations prevent reclamation.
5. Connect/disconnect a display, confirm a layout, reconnect, suspend/resume, and repeat while locked. Verify at least one usable output, mode fallback, rollback and window relocation. Check that ordinary application windows remain accessible through labwc.
6. Start two audio applications; choose headphones, change stream volumes, mute, disconnect hardware and refresh. Confirm failed writes are visible and defaults do not falsely claim to reroute application-specific choices.
7. Send an actionable notification, mute that application, pause notifications, restart the shell and wait for timed expiry. Check lock suppression, disappearing senders, replaced notification IDs and history actions.
8. Capture full/region screenshots, cancel selection, crop, draw/undo, copy, save a copy and cancel close with unsaved edits. Confirm exported dimensions and original preservation; handle missing capture tools and denied save locations.
9. Undo desktop moves and pin changes, including changes from a second desktop surface. Check exhausted history and file-operation boundaries.
10. Capture real current-build screenshots of desktop, both Settings themes, Accessibility, Recovery, Control Center mixer, notification history/toast and screenshot editor. Record revision, monitor geometry, scale and real/sample-data status. Keep old captures labeled historical until these exist.
