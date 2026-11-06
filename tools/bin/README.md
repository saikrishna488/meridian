# Precompiled first-launch installer

`./install.sh` opens `meridian-installer-linux-<uname -m>` from this directory
before considering local developer builds. It checks the adjacent SHA-256 file
and never invokes Cargo just to open the GUI. Keep the executable bit when
copying or publishing the checkout. Checksums detect corruption; use trusted
source/release downloads.

The included Linux x86_64 binary is a native GTK application, not a static or
universal Linux bundle. It needs a graphical session, GTK 4.18 or newer,
GLib 2.78 or newer, and glibc 2.39 or newer. It does not require Rust, Cargo,
Node, WebKit, or GTK development headers just to open. Older distributions
may need a binary built on their own baseline. No aarch64 binary is included
yet; the launcher reports this rather than trying an incompatible executable.

If GTK runtime libraries are missing, install the runtime package from a real
terminal first (Debian/Ubuntu: `sudo apt-get install libgtk-4-1`). The package
must meet the versions above. Installation operations also need sudo in CLI
mode or pkexec and a working desktop Polkit agent in GUI mode. On Debian/Ubuntu,
`sudo apt-get install pkexec mate-polkit` installs the graphical authorization
prerequisites; start the agent or sign out and back in as appropriate for your
desktop. Opening the GUI itself does not request administrator access.

Refresh the bundled binary after changing installer source:

```sh
bash tools/package-installer.sh
```

This builds only `meridian-installer` with one Cargo job, then copies the
executable and writes its checksum. It does not build the web UI or run tests.
Normal release compilation also refreshes this bundle. Generated files here
are excluded from the source-before/source-after comparison, and included in
the final release integrity manifest.
