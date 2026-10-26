# Installation

## Debian package

Build on a supported Debian development host as your regular user:

```sh
tools/build-deb.sh --output /tmp/meridian.deb
sudo apt install /tmp/meridian.deb
```

Without `--output`, the package is written to
`target/meridian_<version>_<architecture>.deb`. Use `--no-build` to package an
existing verified release produced by `tools/build-release.sh`.

The package includes compiled binaries, UI, assets, compositor and greeter
configuration, and the Meridian login-session entry. APT installs the runtime
dependencies and multimedia codecs. The destination needs the build's
architecture, GTK >= 4.18, WebKitGTK >= 2.52, and gtk4-layer-shell >= 1.2.
Rust, Node, and a source checkout are not needed on the destination.

Select Meridian from the existing login screen. To update, build and install the
new package, then sign out and back in. To remove it:

```sh
sudo apt remove meridian-desktop
```

## Source installation and login-screen controls

Developers can install directly from a verified source checkout instead:

```sh
tools/build-release.sh
sudo tools/install.sh
```

Remove a source installation with `sudo tools/install.sh --uninstall`. Keep source
and APT installations separate: use the same method to update and remove an
installation.

Both methods keep the existing login screen by default. From a source checkout,
`tools/install.sh` also provides optional controls for the installed greeter:

```sh
sudo tools/install.sh --try-login-screen     # trial on VT 7
sudo tools/install.sh --enable-login-screen  # use greetd after reboot
sudo tools/install.sh --disable-login-screen # restore the previous login screen
```

Restore the previous login screen before removing a package if you enabled greetd.

`debian.sh` and `codecs.sh` contain shared package lists; `release-manifest.sh`
verifies matching source, binaries, and UI; `install-transaction.sh` restores files
after a failed source installation. These are required helpers, not additional
installer entry points. `install-codecs.sh` remains available for optional codec
installation on a source-installed system.
