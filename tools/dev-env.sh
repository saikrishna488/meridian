#!/usr/bin/env bash
# Create the `meridian-dev` toolbox container with everything needed to build
# and run Meridian. Nothing is installed on the host.
set -euo pipefail
source "$(dirname "$0")/lib.sh"

PACKAGES=(
    rust cargo rustfmt clippy          # Rust toolchain
    nodejs npm                         # TypeScript build for ui/
    gtk4-devel webkitgtk6.0-devel      # WebView host (ADR-0001)
    gtk4-layer-shell-devel             # layer-shell surfaces
    labwc xorg-x11-server-Xwayland     # session compositor (ADR-0002) + X11 apps
    rsms-inter-fonts                   # UI typeface
    foot grim                          # a launchable terminal; screenshots for dev
    wireplumber                        # wpctl: volume control (Options menu)
)

if ! toolbox list --containers | grep -q " $TOOLBOX "; then
    toolbox create -y "$TOOLBOX"
fi
toolbox run -c "$TOOLBOX" sudo dnf install -y --setopt=install_weak_deps=False "${PACKAGES[@]}"
echo "Toolbox '$TOOLBOX' ready. Next: tools/dev-session.sh"
