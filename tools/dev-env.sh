#!/usr/bin/env bash
# Install the native Debian development environment.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tools/debian.sh"
source "$ROOT/tools/codecs.sh"
require_debian
runner=()
[ "$(id -u)" = 0 ] || runner=(sudo)
"${runner[@]}" apt-get update
"${runner[@]}" apt-get install -y --no-install-recommends "${BUILD_PACKAGES[@]}" "${RUNTIME_PACKAGES[@]}" "${MEDIA_PACKAGES[@]}" foot grim
pkg-config --atleast-version=4.18 gtk4 && pkg-config --atleast-version=2.52 webkitgtk-6.0 && pkg-config --atleast-version=1.2 gtk4-layer-shell-0 || {
    echo 'This checkout needs GTK >= 4.18, WebKitGTK >= 2.52 and gtk4-layer-shell >= 1.2. Use a Debian release with these packages (currently testing), or compatible backports.' >&2
    exit 1
}
echo 'Development packages installed. Rust >= 1.92 and Node >= 22.22.2 are required. Next: tools/dev-session.sh'
