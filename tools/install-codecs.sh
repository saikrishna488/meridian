#!/usr/bin/env bash
# Install multimedia codecs from the configured distribution repositories.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tools/distro.sh"
[ "$(id -u)" = 0 ] || { echo "Run as root: sudo $0" >&2; exit 1; }
case "${1:-}" in ""|--full) ;; *) echo "Usage: $0 [--full]" >&2; exit 1 ;; esac
meridian_load_packages
meridian_install_packages -- "${MEDIA_PACKAGES[@]}"
echo 'Multimedia codecs installed. Restart your video player or browser.'
