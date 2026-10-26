Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# System management and session notes

Meridian's production session launches Kitty for terminal work. The embedded
PTY terminal and its scrollback UI described in older development notes are
not part of the current UI bundle; terminal behavior and shortcuts are provided
by Kitty.

## Wi-Fi and Bluetooth

The Wi-Fi page loads the active connection and saved NetworkManager profiles
when opened. Nearby access points are listed only after **New connection** is
selected; the shell requests an explicit NetworkManager rescan at that point.
Bluetooth loads paired and connected BlueZ devices immediately and starts
temporary discovery only after **Pair new device** is selected. Discovery ends
after the scan completes. Both pages offer a separate scan-again action after
the first search.

`tools/compile_meridian.sh` builds the pinned `wayland-pipewire-idle-inhibit`
helper from source and `tools/install.sh` installs it. It inhibits automatic
locking and display standby while PipeWire media audio is active; ordinary
inactivity timers resume when playback stops.
Silent video or media routed outside PipeWire cannot be detected by this
audio-based helper. Wayland-native apps can also inhibit idle through the
compositor protocol.
Manual, idle and pre-suspend locks all open Meridian's own compositor-enforced
lock surface on each display; the current user's password is checked through
the installed `meridian-lock` PAM service.

The Bluetooth page documents the A2DP codec families provided by PipeWire's
BlueZ monitor: SBC, SBC-XQ, AAC, aptX and its HD/LL variants, FastStream,
LDAC, LC3, LC3plus H3 and the Opus 0.5 profiles. It reports the active codec
when PipeWire exposes it, and provides Auto and Highest quality preferences.
Highest quality requests LDAC's high-quality bitrate; it requires a compatible
headset and installed codec support, and may stutter at range. Auto retains
adaptive LDAC quality. The codec list's quality, latency and range notes are
common tradeoffs, not per-device measurements.

Settings includes three additional categories:

- **Device Manager** groups hardware by category, with properties and relevant
  controls in a right-hand pane. Network adapters use NetworkManager; Bluetooth
  adapters use BlueZ; printer queue controls use CUPS with authentication. Storage
  shows driver details and opens the selected disk in GNOME Disks. USB and PCI
  devices expose their bindings and hardware IDs. Unavailable services do not
  offer controls. Kernel modules are not unloaded from this UI.
- **Users** uses AccountsService over the system D-Bus for local accounts, account
  creation, standard/administrator roles, login lock/unlock, password changes and
  deletion. Delete keeps home files. Current-account deletion, locking and
  demotion are blocked; at least one unlocked administrator is retained. Account
  operations are serialized in the shell. Changes from other administration tools
  can still race; refresh to see their results. Existing sessions can survive
  locking and retain group privileges until logout. Passwords are hashed using
  OpenSSL SHA-512 crypt over stdin, never command arguments, and sensitive bridge
  request Debug output is redacted. Setting a password unlocks an account. If
  creation succeeds but password setup fails or is cancelled, the retained account
  is reported and its password can be set from the list.
- **General** shows Meridian version, OS, kernel, architecture, computer name,
  manufacturer, model, processor, and memory. Manual update checks use the public
  GitHub latest stable release API for `saikrishna488/meridian`. Semantic versions
  are compared numerically. Missing/private releases, network errors, rate limits
  and non-semantic tags are reported distinctly. There is no automatic download,
  install, distro upgrade or private-repository token storage.

Driver and account operations have a separate bridge capability available only
in the Settings surface. A GNOME PolicyKit authentication agent starts with labwc;
the installer includes it, AccountsService, kmod, OpenSSL, curl and CA certificates.
Existing sessions need a sign-out/sign-in after installation to start that agent.

## Apply changes from an existing Meridian session

Save work first, then run:

```sh
cd ~/meridian
./tools/compile_meridian.sh
sudo ./tools/install.sh
```

After installation, use **Meridian → Sign out**, confirm, and select **Meridian**
on the login screen. This loads the new native binary, UI, compositor settings
and authentication agent together. Building alone does not replace the installed
session. No reboot is required for these shell changes.

Verify with **Super+I**:

1. **General:** inspect system details, select Check for updates, and open releases.
2. **Device Manager:** select adapters, printers and storage; inspect their driver
   details and exercise the relevant controls using nonessential devices.
3. **Users:** use a temporary standard account to exercise creation, password reset,
   lock/unlock and deletion. Verify the delete dialog says home files are kept.
4. **Kitty:** use Super+Enter to open the installed terminal and confirm it
   starts as an independent application in the Meridian session.

A read-only browser preview is also available without installing or signing out:

```sh
node tools/preview-ui.mjs
```

Open http://127.0.0.1:4173. Driver/account mutation requests are rejected by its
fixture bridge. Update results and system information are sample data.

## Verification limits

Rust tests cover driver inventory merging/input validation, account input
validation and update response/version handling. UI interaction tests cover
management confirmations, protected account rows, password matching, and manual
update checks. The build includes TypeScript checking;
Clippy and formatting checks cover native changes. This environment blocks local
HTTP listeners and browser startup, so visual checks and real PolicyKit,
AccountsService mutations, module changes, and Kitty launch behavior must
be verified in the installed desktop. Tests do not alter real users or drivers.
