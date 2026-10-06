#!/usr/bin/env bash
# Try the login screen in a nested window, against a *fake* greetd (password
# "meridian"); nothing is logged in or started. Closing the window ends it.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"

build_all --bin meridian-greeter
export MERIDIAN_UI_DIR="$ROOT/ui/dist" MERIDIAN_ASSETS_DIR="$ROOT/assets"
export RUST_LOG="${RUST_LOG:-info}"
# Offer this repo's Meridian session plus the host's installed sessions.
export MERIDIAN_SESSION_DIRS="$ROOT/session:/run/host/usr/share/wayland-sessions:/usr/share/wayland-sessions"
exec_compositor python3 "$ROOT/tools/fake-greetd.py" \
    labwc -C "$ROOT/session/greeter/labwc" -S "$ROOT/target/debug/meridian-greeter"
