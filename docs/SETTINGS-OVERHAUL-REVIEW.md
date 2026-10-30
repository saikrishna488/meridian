# Settings and Notifications review

The 200% failure was reproduced with a real compositor pointer and traced with
Wayland protocol logs. In affected runs the transparent GTK layer windows
acknowledged configuration but submitted no initial buffer, so neither wallpaper
nor dock received output-enter or input. The native host now renders a transparent
one-pixel Cairo node for the first two frames and removes it. It unregisters WebKit
message handlers on teardown and cancels pending bootstrap callbacks on unmap.
The original pointer pass-through and hover assertions remain; integration tests
also require the first native buffer and exercise the dock on both outputs.

Storage rejects unknown mount metadata, serializes mutations across both storage
interfaces, and checks disk sequence identity on new-page operations when the
kernel exposes it. The legacy request without an identity remains compatible;
on kernels without disk sequence support, validation is best effort. A device can
still disappear after validation, and UDisks remains the authority for the final
operation. File-manager activation is asynchronous and bounded. Subprocess stdin,
stdout and stderr are drained without blocking the operation deadline. Failed
display rollbacks retain their trial for retry, with errors shown in the dialog.
Bluetooth refresh clears recovered errors, backs off on unavailable services,
and avoids a second request when no adapter or service is available. Bluetooth
scan-stop failures are reported rather than discarded.

The bindings check runs two independent ts-rs exports and compares their exact
contents with the working tree, including obsolete files. Its regression tests
cover changed, missing, unexpected, empty and nondeterministic outputs. It does
not require a commit. `cargo test -p meridian-protocol` regenerates the bindings.

Validation ran on Fedora 44 AArch64 with headless labwc and WebKitGTK software
rendering. `tools/check.sh` passed release-integrity checks, rustfmt, strict
clippy, Rust tests, generated bindings, TypeScript, production build and UI tests.
The full native desktop integration suite passed at 100%, 125%, 150% and 200%,
with both outputs tested. The isolated test's built-in scaler uses the compositor
protocol and refuses non-headless outputs. The accessibility bus could not start.

[Performance samples](performance/settings-review-2026-10-30.json) record CPU,
PSS and bridge request counts. These are sequential whole-shell process-tree
measurements, including retained WebKit renderers and page settling, not isolated
page memory or a before/after benchmark. Storage and Notifications made no bridge
requests during their five-second samples; Bluetooth made one capability request
in 20 seconds with BlueZ unavailable. The display sample included one brightness
probe. CPU and memory results should not be interpreted as a production hardware
performance budget.

Physical drives, PolicyKit, NetworkManager/rfkill, BlueZ pairing and multi-adapter
hotplug, physical display/DDC, real VM resizing, screen readers, and other Linux
distributions still require manual testing. Native hardware and cross-distribution
compatibility have not been certified. No commit was created.
