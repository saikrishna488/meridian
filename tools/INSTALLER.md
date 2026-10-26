Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# Installation

Meridian is currently installed from a source checkout; Debian packages are not
provided. `tools/install.sh` installs runtime dependencies with APT, then copies
the verified release binaries, UI, assets, compositor configuration, PAM
service, and session entry into system locations.

On Debian testing or a compatible Debian derivative:

```sh
tools/dev-env.sh
tools/compile_meridian.sh
sudo tools/install.sh
```

`tools/dev-env.sh` installs the build dependencies. `tools/compile_meridian.sh`
builds Meridian and the pinned PipeWire idle-inhibit helper. The installer
checks that the release matches the source tree before changing the system.
APT still supplies system runtime libraries and services.

Run `dev-env.sh` and `compile_meridian.sh` as your regular user; `dev-env.sh`
uses `sudo` for APT when needed. Only `install.sh` needs `sudo`. The build
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
`debian.sh`, `codecs.sh`, `release-manifest.sh`, and
`install-transaction.sh` are helpers used by the source installer and build
scripts, not separate installer entry points.
