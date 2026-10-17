# Device Manager, Users, General, and terminal scrolling

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

Terminal handles normal-buffer wheel units directly, including fractional
trackpad movement. It retains xterm handling for alternate-screen and mouse-aware
programs. Shift+PageUp/PageDown scroll history; Shift+Home/End go to the beginning
or end. A Jump to latest output button appears while reading scrollback. The
scrollbar has a visible draggable thumb, and unchanged resize events no longer
resize the emulator.

## Apply changes from an existing Meridian session

Save work first, then run:

```sh
cd ~/meridian
./tools/build-release.sh
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
4. **Terminal:** run `seq 1 300`, scroll up with wheel/touchpad and Shift+PageUp,
   then select Jump to latest output or press Shift+End. Test scrollbar dragging,
   resizing, and normal scrolling in a pager such as `less`.

A read-only browser preview is also available without installing or signing out:

```sh
node tools/preview-ui.mjs
```

Open http://127.0.0.1:4173. Driver/account mutation requests are rejected by its
fixture bridge. Update results and system information are sample data.

## Verification limits

Rust tests cover driver inventory merging/input validation, account input
validation and update response/version handling. UI interaction tests cover
confirmation, protected rows, password matching, manual update checks, wheel
normalization and history navigation. The build includes TypeScript checking;
Clippy and formatting checks cover native changes. This environment blocks local
HTTP listeners and browser startup, so visual checks and real PolicyKit,
AccountsService mutations, module changes and WebKitGTK hardware scrolling must
be verified in the installed desktop. Tests do not alter real users or drivers.
