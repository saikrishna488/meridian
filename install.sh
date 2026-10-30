#!/usr/bin/env bash
# Meridian source installer. Run as a regular user from a source checkout.
# The existing transactional tools/install.sh performs all privileged writes.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$ROOT/tools/distro.sh"

usage() {
  cat <<'HELP'
Meridian source installer

Usage: bash install.sh [--install | --check | --update | --repair | --uninstall] [--yes]

  --install     Install from this checkout (default): dependencies, build, install.
  --check       Read-only preflight; report compatibility and build prerequisites.
  --update      Rebuild this checkout and reinstall (does NOT run git pull).
  --repair      Rebuild this checkout and reinstall missing/damaged files.
  --uninstall   Remove Meridian-managed files; keep personal settings and dependencies.
  --yes, -y     Confirm the system package installation prompt (for automation).
  --help, -h    Show this help.

Run without sudo. The script elevates only package installation and the
transactional system install. It never replaces your display manager.
HELP
}

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
note() { printf '\n==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

mode=install
yes=0
mode_seen=0
while (($#)); do
  case "$1" in
    --install|--check|--update|--repair|--uninstall)
      ((mode_seen == 0)) || die "Choose only one installation mode"
      mode="${1#--}"; mode_seen=1 ;;
    --yes|-y) yes=1 ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; die "Unknown argument: $1" ;;
  esac
  shift
done

(( EUID != 0 )) || die "Run as your regular user, not sudo. Privileges are requested only when needed."
[[ -f "$ROOT/Cargo.toml" && -f "$ROOT/tools/install.sh" ]] || die "Run from a complete Meridian source checkout."
[[ "$(uname -s)" == Linux ]] || die "Meridian requires Linux."
command -v sudo >/dev/null 2>&1 || { [[ $mode == check ]] || die "sudo is required for system installation"; }
meridian_load_packages || die "Unsupported distribution. See tools/INSTALLER.md."
note "Meridian source installer • $DISTRO • $(uname -m)"

# Non-mutating preflight: report missing tools and library versions without
# running a package manager, building, or requesting root privileges.
preflight() {
  local problems=0 version
  note "Checking development and runtime requirements (no changes will be made)"
  for tool in bash git cargo rustc node npm pkg-config cc; do
    if command -v "$tool" >/dev/null 2>&1; then
      printf '  [ok] %s\n' "$tool"
    else
      printf '  [missing] %s\n' "$tool"
      problems=1
    fi
  done
  if command -v rustc >/dev/null 2>&1; then
    version="$(rustc --version | awk '{print $2}')"
    if [[ "$(printf '%s\n' 1.92.0 "$version" | sort -V | head -n 1)" != 1.92.0 ]]; then
      warn "Rust $version is older than 1.92"; problems=1
    fi
  fi
  if command -v node >/dev/null 2>&1; then
    version="$(node --version | sed 's/^v//')"
    if [[ "$(printf '%s\n' 22.22.2 "$version" | sort -V | head -n 1)" != 22.22.2 ]]; then
      warn "Node $version is older than 22.22.2"; problems=1
    fi
  fi
  if command -v pkg-config >/dev/null 2>&1; then
    local spec
    for spec in 'gtk4:4.18' 'webkitgtk-6.0:2.52' 'gtk4-layer-shell-0:1.2' 'libpipewire-0.3:0.3'; do
      local pkg="${spec%%:*}" minimum="${spec#*:}"
      if pkg-config --atleast-version="$minimum" "$pkg"; then
        printf '  [ok] %s >= %s\n' "$pkg" "$minimum"
      else
        printf '  [missing/old] %s >= %s\n' "$pkg" "$minimum"
        problems=1
      fi
    done
  else
    problems=1
  fi
  if command -v df >/dev/null 2>&1; then
    local available
    available="$(df -Pk "$ROOT" | awk 'END {print $4}')"
    if [[ "$available" =~ ^[0-9]+$ && "$available" -lt 4194304 ]]; then
      warn "Less than 4 GiB free on the build filesystem; a release build may fail."
    fi
  fi
  if ((problems)); then
    warn "Prerequisites are missing or outdated. Run: bash install.sh --install"
    return 1
  fi
  note "Preflight passed."
}

case "$mode" in
  check)
    preflight
    exit 0
    ;;
  uninstall)
    note "Uninstalling Meridian (personal settings and dependency packages are preserved)"
    sudo "$ROOT/tools/install.sh" --uninstall
    exit 0
    ;;
esac

# Source upgrades are explicit: never fetch or reset a user's working tree.
if [[ "$mode" == update ]]; then
  note "Updating from the CURRENT checkout; run git pull yourself for newer source."
fi

if (( ! yes )); then
  printf '\nThis will install or update system packages using %s and install Meridian\n' "$(
    case "$DISTRO" in debian) echo APT;; fedora) echo DNF;; arch) echo pacman;; esac
  )"
  printf 'under /usr/local. On Arch, the package step may upgrade the system.\n'
  printf 'Your existing login manager will NOT be changed.\n'
  if [[ ! -t 0 ]]; then
    die "Confirmation requires a terminal. Pass --yes only if you approve these changes."
  fi
  read -r -p "Continue? [y/N] " reply
  case "$reply" in y|Y|yes|YES) ;; *) die "Cancelled without installing." ;; esac
fi

note "Installing development and runtime dependencies"
"$ROOT/tools/dev-env.sh"
note "Compiling and verifying the Meridian release as $(id -un)"
"$ROOT/tools/compile_meridian.sh"
note "Checking release integrity"
"$ROOT/tools/install.sh" --check-build
note "Installing the release transactionally (sudo)"
sudo "$ROOT/tools/install.sh"
note "Installation complete"
printf 'Select Meridian from your existing login screen after signing out.\n'
printf 'To update from newer code: git pull, then bash install.sh --update\n'
printf 'To remove Meridian: bash install.sh --uninstall\n'
