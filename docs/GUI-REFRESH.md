Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# Desktop GUI refresh

> Historical implementation notes. Later releases made Meridian dark-only and
> removed the embedded Orbit and terminal surfaces from the production UI
> bundle. Current file browsing and interactive shell sessions use GNOME Files
> and Kitty. See the [current README](../README.md) for shipped behavior.

This pass brings Settings, Orbit, Applications, and Notifications onto the same
light/dark palette, with calmer surfaces, consistent focus outlines, readable
headings, and grouped navigation. Native window controls remain in GTK.

## Daily use

Settings separates Workspace, Devices & connections, and System. The Home page
links to frequently used preferences; its theme button switches the saved desktop
appearance. Search includes microphone, startup, battery, and idle-lock terms.
The layout adapts to narrow windows without losing category access.

Orbit has a navigation rail, path breadcrumbs, list column labels, file-type
artwork, and an explicit sort direction. Grid/list view, sort key/direction, and
hidden-file visibility are remembered in the WebView's storage. A debounced native
folder monitor refreshes external changes and retains the current selection. Properties are
read-only and include size, location, dates, owner/group, permissions, and link
targets; folders are not recursively scanned to estimate size. Paths and other
property values can be selected and copied with the keyboard.

Applications can filter to pinned taskbar favorites and follows live pin changes.
Notifications groups messages into Today and Earlier, keeps app actions, and
avoids replacing a newer incoming message with a stale dismissal response.
Opening the center marks messages read; clearing the history publishes one update.

## New system integrations

### Login applications

Settings → Startup Apps reads the effective XDG autostart entries, including
per-user overrides. Switches write to the current user's `autostart` directory;
system files are never changed. Installed apps can be added through a picker.
No application is launched by editing this preference.

The session helper invokes `meridian-shell --autostart` once per compositor
session. It uses GIO desktop-entry launching (including Exec field expansion),
checks desktop filters and TryExec, and skips disabled entries. Development
sessions with `MERIDIAN_DEV=1` skip startup apps. Shell reactivation does not
launch them again. Invalid overriding files do not fall through to a system copy.

Reference: [XDG Autostart specification](https://specifications.freedesktop.org/autostart/latest/).
Management has a dedicated bridge capability granted only to Settings.

### Audio

Sound discovers input and output nodes from PipeWire's JSON snapshot and uses
WirePlumber for default-device, mute, and level changes. Application streams and
monitor sources are excluded. Before a write, the host resolves the node again
and verifies its name, so a stale/reused numeric device ID is rejected. Commands
use fixed argument vectors with bounded waits. Volume is limited to 0–100%.
Meridian does not record microphone samples or play a test sound.

Device discovery is refreshed explicitly. Existing apps may maintain their own
routes. The older output-volume control remains available if device discovery is
unavailable and the volume service still works.

Reference: [PipeWire pw-dump](https://pipewire.pages.freedesktop.org/pipewire/page_man_pw-dump_1.html).

### Power and screen locking

Battery and external-power readings come from the kernel's `power_supply` class.
Missing readings stay unavailable, and peripheral batteries are excluded. The
panel shows a battery button only when a system battery is present.

Power & Lock saves idle timing to the user's `meridian/power.json`. Supported lock
delays are 1, 2, 5, 10, 15, and 30 minutes; standby follows 30 seconds, 1 minute,
2 minutes, or 5 minutes later. The native reader and shell helper validate saved
values; malformed settings fall back to five-minute locking and 30-second standby
grace. Changes apply at the next sign-in. Existing before-sleep locking and
lock-confirmation checks remain active.

### Trash

Orbit uses GIO's `trash:///` namespace, including GVfs support for mounted-volume
trash. Restore accepts only exact item URIs returned by the host's current trash
listing. It moves the item to its original path without overwrite flags. A name
conflict or missing parent folder leaves the item in Trash and reports the issue.
Non-UTF-8 original paths are shown as unavailable rather than round-tripped through
JSON. The browser displays up to 5,000 items and does not permanently delete files.

## Keyboard and accessibility

- Keyboard focus remains visible across desktop controls and menus.
- Reduced-motion preferences disable decorative transitions and animations.
- Dialogs trap focus and return it to their trigger on close.
- Orbit supports Alt+Enter (properties), Alt+Up (parent), Ctrl+Shift+N (new folder),
  Ctrl+L (path), Ctrl+F (search), and the existing file shortcuts. Terminal tab
  shortcuts are intercepted before control characters can reach the shell, and
  terminal text size is remembered.
- Custom dropdowns expose combobox/listbox roles and keep native form semantics.
- Battery meters and controls expose names and current values to assistive tools.

## Verification

The existing test suite is extended with startup picker/switch flows, audio device
and microphone actions, Trash restore/conflict handling, file properties and
breadcrumbs, live pinned filtering, power preferences, battery navigation, and
stale notification response coverage. Rust fixtures cover startup precedence and
filters, no-overwrite restoration, link properties, PipeWire parsing, and missing
battery readings. Shell tests cover valid and malformed idle timing.

Visual previews use deterministic sample data in WebKit on a private headless
labwc session. They do not exercise hardware mutations or real account settings.
See [the screenshot gallery](previews/README.md).
