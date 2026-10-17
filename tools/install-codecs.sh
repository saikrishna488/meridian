#!/usr/bin/env bash
# Install multimedia codecs from the configured Debian repositories.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tools/debian.sh"
source "$ROOT/tools/codecs.sh"
[ "$(id -u)" = 0 ] || { echo "Run as root: sudo $0" >&2; exit 1; }
case "${1:-}" in ""|--full) ;; *) echo "Usage: $0 [--full]" >&2; exit 1 ;; esac
require_debian
apt-get update
apt-get install -y --no-install-recommends "${MEDIA_PACKAGES[@]}"
echo 'Multimedia codecs installed. Restart your video player or browser.'
