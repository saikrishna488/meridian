#!/usr/bin/env bash
# Build Meridian and run it in a nested labwc window inside the current
# Wayland desktop. Closing the window (or the shell exiting) ends the session.
#
#   tools/dev-session.sh            # debug build, dev mode (inspector enabled)
#   tools/dev-session.sh --release
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"

profile=debug
cargo_args=()
if [ "${1:-}" = "--release" ]; then
    profile=release
    cargo_args=(--release)
fi

build_all "${cargo_args[@]}"

# Install only Meridian's own compositor theme for this development session.
theme_root="${XDG_DATA_HOME:-$HOME/.local/share}/themes/Meridian"
mkdir -p "$theme_root"
cp -r "$ROOT/assets/themes/Meridian/." "$theme_root/"

if [ -z "${WAYLAND_DISPLAY:-}" ]; then
    echo "error: run this from a Wayland session (nested labwc needs a host compositor)" >&2
    exit 1
fi

export XDG_CURRENT_DESKTOP="Meridian:wlroots"
export MERIDIAN_DEV="${MERIDIAN_DEV:-1}"
export RUST_LOG="${RUST_LOG:-info}"
export MERIDIAN_UI_DIR="$ROOT/ui/dist"
export MERIDIAN_ASSETS_DIR="$ROOT/assets"

# In a toolbox, show and launch the *host's* applications: the shell reads
# the host's desktop entries through /run/host and launches them on the host
# via flatpak-spawn (see services/src/apps.rs, HostBridge).
if [ -e /run/.toolboxenv ]; then
    export MERIDIAN_HOST_APPS=1
fi

# GApplication is unique on the session bus. A previous nested shell (or
# an installed Meridian session) must not absorb this shell's activation.
# Include labwc so its gapplication keyboard bindings share the same bus.
compositor=(dbus-run-session -- labwc -C "$ROOT/session/labwc" -S "$ROOT/target/$profile/meridian-shell")

exec_compositor "${compositor[@]}"
