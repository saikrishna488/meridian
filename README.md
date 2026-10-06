# Meridian

A minimal, text-only Linux desktop shell. It has a single top bar:
**Applications** (search plus a list of your apps), your **running windows**,
and **Options** (Wi-Fi, Bluetooth, brightness, volume). There are no icons.

Meridian is built on mature Linux infrastructure: Wayland (labwc today),
D-Bus services, and XDG desktop entries. The native parts are in Rust, and
the UI is HTML/CSS/TypeScript in a locked-down WebKit host.

**Status:** Milestone 0, a working prototype. See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Try it

```sh
tools/dev-env.sh        # one-time: create the `meridian-dev` toolbox (nothing installed on the host)
tools/dev-session.sh    # build and run Meridian in a nested window
tools/dev-greeter.sh    # try the login screen in a window (fake login, password "meridian")
tools/check.sh          # fmt, clippy, tests, UI type-check
```

## Install (Fedora)

```sh
tools/build-release.sh                       # as your user
sudo tools/install.sh                        # adds "Meridian" to your login screen's session list
sudo tools/install.sh --try-login-screen     # optional: Meridian's login screen on Ctrl+Alt+F7
sudo tools/install.sh --enable-login-screen  # optional: make it your login screen (after reboot)
sudo tools/install.sh --disable-login-screen # undo the above
sudo tools/install.sh --uninstall
```

| Keys | Action |
|---|---|
| Super / Super+Space | Applications menu (type to search, ↑ ↓, Enter, Esc) |
| Super+C | Options menu |
| click a window name in the bar | focus that window |
| Alt+Tab | switch windows |
| Super+Shift+Q / Ctrl+Alt+Backspace | log out |

In the nested dev session the host desktop usually grabs Super, so use
**Ctrl+Alt+H** (Applications) and **Ctrl+Alt+C** (Options), or click the bar.

Documentation: [architecture](docs/ARCHITECTURE.md) ·
[development](docs/DEVELOPMENT.md) · [decision records](docs/adr/)

## License

Meridian is free software under the [GNU GPL, version 3 or later](LICENSE).
