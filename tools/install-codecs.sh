#!/usr/bin/env bash
# Fedora codecs only: sudo tools/install-codecs.sh
# Broader codec support from RPM Fusion Free: sudo tools/install-codecs.sh --full
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tools/codecs.sh"
[ "$(id -u)" = 0 ] || { echo "Run as root: sudo $0 ${1:-}" >&2; exit 1; }
case "${1:-}" in ""|--full) ;; *) echo "Usage: $0 [--full]" >&2; exit 1 ;; esac
source /etc/os-release
[ "${ID:-}" = fedora ] || { echo "This installer supports Fedora." >&2; exit 1; }

media_packages_for_host
dnf install -y --setopt=install_weak_deps=False "${MEDIA_SELECTED[@]}"

if [ "${1:-}" = --full ]; then
    fedora_version="$(rpm -E %fedora)"
    [[ "$fedora_version" =~ ^[0-9]+$ ]] || { echo "Cannot determine Fedora version." >&2; exit 1; }
    if ! rpm -q rpmfusion-free-release >/dev/null 2>&1; then
        dnf install -y "https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-${fedora_version}.noarch.rpm"
    fi
    # Preserve an existing full FFmpeg stack; otherwise supplement Fedora's.
    full_packages=(gstreamer1-plugins-bad-freeworld)
    if ! rpm -q ffmpeg-libs >/dev/null 2>&1; then full_packages+=(libavcodec-freeworld); fi
    dnf install -y --setopt=install_weak_deps=False "${full_packages[@]}"
    if rpm -q gstreamer1-plugins-ugly-free >/dev/null 2>&1; then
        dnf swap -y gstreamer1-plugins-ugly-free gstreamer1-plugins-ugly
    else
        dnf install -y gstreamer1-plugins-ugly
    fi
fi
echo "Multimedia codecs installed. Restart your video player or browser."
