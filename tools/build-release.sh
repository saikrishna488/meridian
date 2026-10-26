Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
#!/usr/bin/env bash
# Build optimized binaries and the UI bundle for Debian and source installation.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"
source "$ROOT/tools/release-manifest.sh"
rm -f "$ROOT/target/release/meridian-build.sha256"
sources_before="$(release_source_manifest)"
build_all --release --bin meridian-shell --bin meridian-greeter
cargo install --locked --version 0.7.1 --root "$ROOT/target/media-idle-inhibit" wayland-pipewire-idle-inhibit
install -Dm755 "$ROOT/target/media-idle-inhibit/bin/wayland-pipewire-idle-inhibit" \
    "$ROOT/target/release/wayland-pipewire-idle-inhibit"
if [ "$sources_before" != "$(release_source_manifest)" ]; then
    echo "error: source files changed during the build; run tools/build-release.sh again" >&2
    exit 1
fi
release_manifest > "$ROOT/target/release/meridian-build.sha256"
echo "Built. Install with: sudo tools/install.sh"
