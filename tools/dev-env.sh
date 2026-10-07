#!/usr/bin/env bash
# Create the `meridian-dev` toolbox container with everything needed to build
# and run Meridian. Nothing is installed on the host.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
source "$ROOT/tools/codecs.sh"

PACKAGES=(
    rust cargo rustfmt clippy          # Rust toolchain
    nodejs npm                         # TypeScript build for ui/
    gtk4-devel webkitgtk6.0-devel      # WebView host (ADR-0001)
    gtk4-layer-shell-devel             # layer-shell + session-lock surfaces (ADR-0007)
    pam-devel                          # lock screen password check (ADR-0007)
    clang-devel                        # libclang: gtk4-session-lock-sys generates bindings at build time
    labwc xorg-x11-server-Xwayland     # session compositor (ADR-0002) + X11 apps
    rsms-inter-fonts                   # UI typeface
    foot grim                          # a launchable terminal; screenshots for dev
    NetworkManager                  # nmcli Wi-Fi operations through the host system bus
    ddcutil                           # DDC/CI brightness for compatible external monitors
    gsettings-desktop-schemas
    wireplumber                        # wpctl: volume control (Options menu)
)

if ! toolbox list --containers | grep -q " $TOOLBOX "; then
    toolbox create -y "$TOOLBOX"
fi
toolbox run -c "$TOOLBOX" sudo dnf install -y --setopt=install_weak_deps=False "${PACKAGES[@]}" "${MEDIA_PACKAGES[@]}" ffmpeg-free
echo "Toolbox '$TOOLBOX' ready. Next: tools/dev-session.sh"
