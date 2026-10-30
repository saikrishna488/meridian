# Installation

Meridian is currently installed from a source checkout; Distribution packages are not
provided. `tools/install.sh` installs runtime dependencies with APT, DNF, or pacman, then copies
the verified release binaries, UI, assets, compositor configuration, PAM
service, and session entry into system locations.

On Debian, Fedora, Arch, or a derivative with the required library versions:

```sh
tools/dev-env.sh
tools/compile_meridian.sh
sudo tools/install.sh
```

`tools/dev-env.sh` installs the build dependencies. `tools/compile_meridian.sh`
builds Meridian and the pinned PipeWire idle-inhibit helper. The installer
checks that the release matches the source tree before changing the system.
The distribution package manager supplies system runtime libraries and services.

Run `dev-env.sh` and `compile_meridian.sh` as your regular user; `dev-env.sh`
uses `sudo` for the package manager when needed. Only `install.sh` needs `sudo`. The build
requires the PipeWire development package installed by `dev-env.sh`.

Select **Meridian** from the existing login screen. To update, rebuild and run
`sudo tools/install.sh` again, then sign out and back in. To remove Meridian:

```sh
sudo tools/install.sh --uninstall
```

The existing login screen remains the default. The installer has optional
controls for trying Meridian's greetd-based login screen on VT 7 or enabling it
as the default:

```sh
sudo tools/install.sh --try-login-screen
sudo tools/install.sh --enable-login-screen
sudo tools/install.sh --disable-login-screen
```

Restore the previous login screen before uninstalling if greetd was enabled.
`distro.sh`, `debian.sh`, `codecs.sh`, `release-manifest.sh`, and
`install-transaction.sh` are helpers used by the source installer and build
scripts, not separate installer entry points.

The host profile is selected from `/etc/os-release` (`ID`, then `ID_LIKE`).
Unknown families stop before installing packages. Debian testing is the current
CI baseline; Fedora and Arch profiles are not yet qualified in full desktop
sessions. Older releases may lack the minimum versions listed in the README.
Build on the target distribution; release binaries are not portable bundles.
Arch setup runs `pacman -Syu` to avoid partial upgrades. Fedora uses only the
configured repositories; codec coverage can vary. No third-party repositories
are added by codec setup.

The session requires systemd/logind and the listed host services. Immutable
systems (including rpm-ostree), NixOS and non-systemd distributions need separate
packaging and are not supported by these installers. Lock authentication uses
Debian's common-auth/common-account, Fedora's system-auth, or Arch's
system-local-login. Verify locking and unlocking in a test session before using
Meridian as your primary desktop. Native application removal in Settings still
supports Debian packages only; Flatpak and Snap removal remain available on
other distributions. GNOME Software can manage other native package formats
when the distribution provides its backend.
