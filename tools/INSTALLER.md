# Installation

Meridian is currently installed from a source checkout; Distribution packages are not
provided. `tools/install.sh` installs runtime dependencies with APT, DNF, or pacman, then copies
the verified release binaries, UI, assets, compositor configuration, PAM
service, and session entry into system locations.

On Debian, Fedora, Arch, or a derivative with the required library versions,
run the source installer as your regular user (**do not run it with sudo**):

```sh
bash install.sh
```

The command opens the native installer. Choose Install to compile and install
in one workflow, or use the CLI commands below. Both interfaces share the same
backend: dependency setup, unprivileged compilation, release integrity checking,
and transactional system installation. Terminal operations use sudo; GUI
operations use desktop Polkit authentication. The login manager remains unchanged.

```sh
bash install.sh --cli --compile     # compile a verified release, no Meridian file installation
bash install.sh --cli --check       # read-only report; no install or build
bash install.sh --cli --update      # install from checked-out source, no git pull
bash install.sh --cli --repair      # rebuild/reinstall from checked-out source
bash install.sh --cli --uninstall   # preserve user preferences and dependencies
bash install.sh --cli --install --yes  # skip confirmation only for reviewed automation
```

Lower-level helpers remain available for compatibility, but the public entry
point for both compilation and installation is `install.sh`:

```sh
bash install.sh --cli --compile   # dependencies and compilation only
bash install.sh --cli --prepared  # install that verified release
```

`tools/dev-env.sh` installs the build dependencies. `tools/compile_meridian.sh`
builds Meridian and the pinned PipeWire idle-inhibit helper. The installer
checks that the release matches the source tree before changing the system.
The distribution package manager supplies system runtime libraries and services.

Run `dev-env.sh` and `compile_meridian.sh` as your regular user; `dev-env.sh`
uses `sudo` for the package manager when needed. Only the internal `tools/install.sh` performs privileged file installation; do not run the public `install.sh` as root. The build
requires the PipeWire development package installed by `dev-env.sh`.

Select **Meridian** from the existing login screen. To update, run `git pull` first if you want newer code, then `bash install.sh --cli --update`,
and sign out and back in. To remove Meridian:

```sh
bash install.sh --cli --uninstall
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

## Native graphical workflow

Run `bash install.sh` as your regular user (the default is GUI; `--gui` is an
explicit alias). Use `bash install.sh --cli` for the terminal interface. The
launcher first opens the checksum-verified native executable in `tools/bin`,
so first launch requires no Rust, Cargo, Node, or development headers. It does
not prefer a potentially stale system-installed installer and does not silently
compile a GUI. Local developer builds are a fallback when no architecture bundle
is present. See `tools/bin/README.md` for runtime and architecture requirements:
the included x86_64 build requires GTK >= 4.18, GLib >= 2.78 and glibc >= 2.39.
This is not a universal/static Linux binary; missing GTK runtime libraries or
unsupported architectures produce an actionable error.
After installation, launch **Meridian Installer** from Applications or use
**Settings → General → Open Installer**.

Choose a trusted source checkout and an action: Install, Update, Repair,
Compile only, Check requirements, or Remove. Install, Update, and Repair compile
and install through the same `install.sh` backend; Compile only stops after
creating the verified release. The last selected checkout is remembered per
user. Update uses the selected checkout exactly as it stands, including local
changes. It does not fetch, pull, reset, or switch Git branches. Removal requires
its own explicit checkbox and preserves personal settings and dependency packages.
The GUI remains open during an operation and bounds its event queue and visible
log buffer; it does not host a browser or a Node process.

**Use prepared release** selects `--prepared`: integrity-check the existing
release and install it without compiling or installing development dependencies.
Stale or incomplete releases are rejected. This is the faster installation path,
not an instant download/update service; runtime package installation can still
take time. The prepared release must have been built for the target distribution.
For example, from a checkout with a current verified release:

```sh
bash install.sh --cli --update --prepared
```

For source builds, the GUI uses `CARGO_BUILD_JOBS=1` and caps the Node V8 heap at
1024 MiB. UI compilation still uses Node/esbuild as a build dependency; these
limits reduce concurrent build pressure rather than imposing a total system
memory ceiling. No tests are run by install/update. Progress is indeterminate
when a step cannot report a percentage; output is streamed with bounded memory.

The GUI sets `MERIDIAN_GUI_AUTH=1`, selecting `pkexec` for package installation
and transactional system writes. A working desktop Polkit authentication agent
and `pkexec` are required; the GUI never asks for, stores, or logs a password.
Terminal installation continues to use sudo. Source builds stay unprivileged,
and install/update leave the login manager unchanged. A per-user lock prevents
concurrent source operations; a root-owned runtime lock serializes system writes.
The installer binary and desktop launcher are covered by the release manifest
and transaction rollback. Sign out and back in after updating Meridian.

## Authentication window closes or `/dev/tty` error

The GUI has no controlling terminal. A `/dev/tty` authentication error usually
means no graphical Polkit agent is registered. A password window that immediately
closes can also indicate an agent crash; check `journalctl -b` for agent errors.
Meridian supervises its agent and prefers MATE's agent over the legacy GNOME
agent. On Debian/Ubuntu, repair the installed agent from a real terminal with
`sudo apt-get install mate-polkit`, then sign out and back in after installing
this version of Meridian. As a temporary alternative, run
`bash install.sh --cli` from the source directory in a terminal; sudo then uses
that terminal. The GUI disables pkexec's internal text-agent fallback and never
automatically repeats cancelled or failed privileged commands.
