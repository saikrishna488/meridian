#!/usr/bin/env bash
# Exercise host selection, package invocation and PAM without changing the host.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "$ROOT/tools/distro.sh"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
export MERIDIAN_OS_RELEASE="$scratch/os-release"
apt-get() { printf 'apt-get'; printf ' <%s>' "$@"; printf '\n'; }
dnf() { printf 'dnf'; printf ' <%s>' "$@"; printf '\n'; }
pacman() { printf 'pacman'; printf ' <%s>' "$@"; printf '\n'; }
sudo() { "$@"; }
for fixture in debian ubuntu fedora arch derivative; do
    case "$fixture" in
        derivative) printf 'ID=custom\nID_LIKE="arch debian"\n' > "$MERIDIAN_OS_RELEASE"; expected=arch ;;
        ubuntu) printf 'ID=ubuntu\nID_LIKE=debian\n' > "$MERIDIAN_OS_RELEASE"; expected=debian ;;
        *) printf 'ID=%s\n' "$fixture" > "$MERIDIAN_OS_RELEASE"; expected=$fixture ;;
    esac
    meridian_load_packages
    [ "$DISTRO" = "$expected" ]
    [ "${#BUILD_PACKAGES[@]}" -gt 0 ]
    [ "${#RUNTIME_PACKAGES[@]}" -gt 0 ]
    [ "${#MEDIA_PACKAGES[@]}" -gt 0 ]
    output=$(meridian_install_packages sudo -- 'one package' two)
    case "$DISTRO" in
        debian)
            [ "$output" = $'apt-get <update>\napt-get <install> <-y> <--no-install-recommends> <one package> <two>' ]
            cmp <(meridian_write_lock_pam) "$ROOT/session/meridian-lock.pam" ;;
        fedora)
            [ "$output" = 'dnf <install> <-y> <one package> <two>' ]
            [ "$(meridian_write_lock_pam)" = $'#%PAM-1.0\nauth include system-auth\naccount include system-auth' ] ;;
        arch)
            [ "$output" = 'pacman <-Syu> <--needed> <--noconfirm> <one package> <two>' ]
            [ "$(meridian_write_lock_pam)" = $'#%PAM-1.0\nauth include system-local-login\naccount include system-local-login' ] ;;
    esac
done
printf 'ID=unknown\nID_LIKE="notdebian archlike"\n' > "$MERIDIAN_OS_RELEASE"
if meridian_load_packages 2>/dev/null; then echo 'Unknown distro was accepted' >&2; exit 1; fi
# Updating metadata must fail closed, without attempting an install.
DISTRO=debian
apt-get() { [ "$1" != update ] || return 42; echo 'unexpected install'; }
if meridian_install_packages -- example > "$scratch/output"; then exit 1; fi
[ ! -s "$scratch/output" ]
echo 'Distro detection, package commands, failure handling and PAM profiles passed.'
