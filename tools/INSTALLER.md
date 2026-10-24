# Single-file installer

Build on Meridian's supported Debian development host:

```sh
bash tools/package-installer.sh
```

Distribute `target/meridian-installer.run`. On a compatible Debian system:

```sh
bash meridian-installer.run --check-build
sudo bash meridian-installer.run
```

No source checkout, Rust, or Node installation is needed on the destination.
The file includes compiled native binaries, the UI, assets, session files, and
installer helpers. Source inputs are included to preserve release verification.
Runtime libraries and codecs are installed from configured APT repositories, so
installation requires network access unless those packages are already cached.
The destination must match the build's architecture and meet Meridian's GTK,
WebKitGTK, and gtk4-layer-shell requirements; this is not a universal Linux binary.

The default action adds Meridian to the login screen's session list. Sign out and
select Meridian. It does not replace the current login screen. Keep the installer
for `--uninstall` and the existing optional login-screen commands (`--help`).

Use `--no-build` only with a verified current release. Use `--output /path/file.run`
to select a destination outside the checkout. The embedded checksum detects
corruption; it is not a publisher signature. Only run installers from a trusted source.
