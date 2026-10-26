Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# Beta 0.8 desktop workflow work

Status: **initial core workflow implementation; Phase 3 is incomplete**. This is
not a Public Beta release. The package version remains unchanged. The Phase 1
security/hardware gates and Phase 2 performance gates remain prerequisites.

> This milestone records an earlier implementation. The Orbit/Finder and
> embedded PTY Terminal interfaces described below remain in legacy source but
> are excluded from the current production UI bundle. The shipped session
> launches GNOME Files and Kitty instead. See the current
> [README](../README.md) and [architecture](ARCHITECTURE.md).

## Implemented workflows

### System Settings

- Removed Sharing from navigation and deleted its simulated switches. No sharing
  services are enabled by this change.
- Displays now saves a connector preference for Meridian's primary output. The
  dock, top bar and menus move immediately. On disconnect, the shell uses an
  available output and retains the preference for reconnection. Automatic mode
  restores first-available-output selection. This is a Meridian surface preference,
  not a new global Wayland primary-display protocol or a promise to reposition
  existing application windows.
- External brightness queries and writes reject stale responses when the selected
  monitor changes. Existing DDC/CI availability checks and display-mode rollback
  trials remain.
- Wi-Fi connection, Bluetooth discovery, pairing, connection and disconnection
  operations identify what is happening in an accessible status message. Existing
  native Bluetooth authentication/confirmation prompts remain responsible for pairing.
- Audio devices refresh when Settings regains focus or becomes visible, with the
  existing pending-operation guard and manual refresh retained. Hardware hotplug
  subscriptions and full Wi-Fi profile management are still pending.
- Search preferences expose local Settings search and an optional calculator.
  Applications remain searchable. Preferences use private, versioned atomic storage.

### Orbit

- Up to twelve tabs share the existing WebView. Each tab keeps its own back/forward
  history; only the selected tab lists and watches a directory. Closing and reopening
  the cached Orbit window retains tabs. Tabs are not persisted across shell/WebKit
  crashes or logout.
- Ctrl+T opens a tab at the current location and focuses its address field. Ctrl+W
  closes the active tab while preserving at least one. Ctrl+Tab/Ctrl+Shift+Tab and
  Ctrl+PageUp/PageDown switch tabs. Tab buttons support arrows, Home and End.
  Tabs cannot switch during a mutation or modal dialog.
- Space, the Preview toolbar button and the context menu show a file preview.
  UTF-8 text is limited to the first 64 KiB, with truncation disclosed. HTML and
  script source render as text, never as a document. Images use the existing
  bounded thumbnail decoder; this is a small image preview, not a full-size editor.
  Binary and special files show an unsupported state. Opening a FIFO does not
  block. Reads run on the native worker pool and retain the Files capability.
- Preview dialogs trap focus and restore it on close. File shortcuts no longer
  consume Enter or Space intended for sidebar and toolbar buttons. Transfer progress
  and cancellation from Phase 1 remain intact.

### Launcher and window management

- Launcher searches thirteen allowlisted Settings pages by names and fuzzy keywords,
  including Wi-Fi, displays, sound, power, users and updates. Activation sends a
  typed page request to Settings. Settings results cannot be dragged, pinned or
  uninstalled as applications.
- The optional calculator accepts bounded arithmetic: parentheses, decimal/scientific
  numbers, unary signs, +, -, *, / and remainder (%). Input is limited to 256 bytes
  and nesting to 32 levels; malformed/non-finite expressions produce no result.
  Activation recomputes the expression and copies its answer. It does not evaluate
  JavaScript or invoke a shell. Provider toggles are enforced natively on activation.
- Dock application groups display a window count. Clicking cycles to the next
  window, including when another window in the group is already active. Window
  operations continue through the existing compositor protocol.

### Notifications and accessibility

- Do Not Disturb is saved across sessions and suppresses unread badges. Existing
  messages become read when enabled; new messages remain available in this session's
  history. Meridian currently has no notification toast layer; the UI explains the
  badge behavior. Notification contents are not persisted by this change.
- History groups by day and application. Action channels are invalidated when their
  D-Bus sender disconnects, so terminated applications do not leave active actions.
  The bounded history and sender/action ownership checks remain in place.
- Added high-contrast and forced-color focus/selection styles. Launcher keyboard
  scrolling observes reduced motion. New controls have accessible labels, and tabs
  and previews have keyboard/focus regression coverage. These checks do not establish
  screen-reader compatibility or font-scaling qualification.

## Validation

The nested fixture exercises typed settings/calculator queries, capability denials,
primary-output changes and restoration on two virtual outputs, an actual Orbit
text preview with escaped markup, two Orbit tabs, and saved Do Not Disturb through
the permitted top-bar bridge. It also retains Phase 1's renderer crash recovery,
lock fallback, terminal paste/scroll, cached-window reuse and independent application
survival when the supervised shell is killed.

The environment uses software rendering, private configuration/data/cache/Desktop
folders, an isolated D-Bus session and unavailable optional system services. It does
not modify the running Meridian installation or qualify physical hardware.

Final automated checks passed: formatting, Clippy with all targets/features,
**206 Rust tests** (four ignored in the ordinary unit run), **94 UI tests**,
and the strict nested desktop scenario. The scenario completed fifty cached-app
reuse cycles plus a sixty-second soak with nine native windows and 56 shell-tree
processes both before and after the soak. Shell RSS changed from 299,400 to
291,188 KiB; summed tree RSS changed from 2,309,484 to 2,289,824 KiB. Summed RSS
counts shared pages repeatedly and this short observation is not a leak proof.
Image/GVfs tests still need their desktop services; screen-reader and physical
hardware qualification remain open.

## Remaining work before Public Beta

| Area | Open implementation and qualification work |
| --- | --- |
| Settings | Saved Wi-Fi profiles/forget/disconnect flows; real Bluetooth pairing scenarios; audio hotplug; input-device configuration; shortcut customization; broader privacy controls; update installation/status integration. |
| Displays | Physical disconnect/reconnect, disabled outputs, suspend/resume, DDC/CI hardware, fractional scale and refresh-rate combinations. Existing window placement remains compositor-owned. |
| Orbit | Dual panes, bulk rename, archive creation/extraction, recursive search, removable-drive mounting/ejecting and network-location management. Preserve staging/no-overwrite/cancellation guarantees when adding these operations. |
| Search | Recent files and recent applications with explicit privacy/clear controls, provider expansion, categorization and large-catalog responsiveness qualification. No history collection has been introduced. |
| Window management | Configurable workspace shortcuts and dock placement/size; multi-monitor window placement, Alt+Tab presentation, fullscreen and position-restoration testing through supported labwc mechanisms. |
| Utilities | Optional persistent notification contents, clipboard history with privacy controls, screenshot annotation, Calendar improvements and additional useful Pulse views. |
| Accessibility | Real screen-reader sessions, font scaling, light/dark and high-contrast visual review, touchpad gestures and a keyboard audit of every existing app. |
| Release | Complete earlier-phase gates, clean install/rollback qualification and extended daily-use hardware sessions. |

The implemented subset does not satisfy all six Phase 3 sections or the Public
Beta completion criteria. See [Phase 1](PHASE1.md) and [Phase 2](PHASE2.md) for
additional release prerequisites.
