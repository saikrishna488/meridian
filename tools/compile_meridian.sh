#!/usr/bin/env bash
# Build optimized binaries and the UI bundle for source installation.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
if [ "$(id -u)" = 0 ]; then
    echo "error: compile as your regular user, without sudo (sudo is used only to install a missing build package)." >&2
    exit 1
fi
ensure_toolchain "$0" "$@"
source "$ROOT/tools/release-manifest.sh"
if ! command -v pkg-config >/dev/null 2>&1 || ! pkg-config --atleast-version=0.3 libpipewire-0.3; then
    echo "==> Installing the missing PipeWire build dependency"
    source "$ROOT/tools/distro.sh"
    meridian_load_packages
    command -v sudo >/dev/null 2>&1 || {
        echo "error: sudo is required to install the PipeWire build dependencies." >&2
        exit 1
    }
    meridian_install_packages sudo -- "${PIPEWIRE_BUILD_PACKAGES[@]}"
    pkg-config --atleast-version=0.3 libpipewire-0.3 || {
        echo "error: The installed packages did not provide the required PipeWire development files." >&2
        exit 1
    }
fi
rm -f "$ROOT/target/release/meridian-build.sha256"
sources_before="$(release_source_manifest)"
build_all --release --bin meridian-shell --bin meridian-greeter
cargo install --locked --version 0.7.1 --root "$ROOT/target/media-idle-inhibit" wayland-pipewire-idle-inhibit
install -Dm755 "$ROOT/target/media-idle-inhibit/bin/wayland-pipewire-idle-inhibit" \
    "$ROOT/target/release/wayland-pipewire-idle-inhibit"
if [ "$sources_before" != "$(release_source_manifest)" ]; then
    echo "error: source files changed during the build; run tools/compile_meridian.sh again" >&2
    exit 1
fi
release_manifest > "$ROOT/target/release/meridian-build.sha256"
echo "Built. Install with: sudo tools/install.sh"
