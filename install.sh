#!/usr/bin/env bash
# Meridian source installer. Run as a regular user from a source checkout.
# The existing transactional tools/install.sh performs all privileged writes.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$ROOT/tools/distro.sh"
source "$ROOT/tools/privilege.sh"

usage() {
  cat <<'HELP'
Meridian source installer

Usage: bash install.sh [--gui] [--install | --update | --repair | --compile | --check | --uninstall]
       bash install.sh --cli [--install | --update | --repair | --compile | --check | --uninstall] [--prepared] [--yes]

  (no flags)    Open the native graphical installer (default).
  --gui         Explicitly select the graphical interface.
  --cli         Use the terminal interface; required for unattended operation.
  --install     Compile and install from this checkout (default action).
  --update      Compile and reinstall this checkout (does NOT run git pull).
  --repair      Compile and reinstall missing/damaged files.
  --compile     Install prerequisites and compile a verified release; do not install Meridian files.
  --check       Read-only preflight in CLI mode; preselect checks in the GUI.
  --uninstall   Remove Meridian-managed files; keep personal settings and dependencies.
  --prepared    Install a verified prepared release without compiling or build dependencies.
  --yes, -y     Confirm package installation in CLI mode (for automation).
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
prepared=0
interface=gui
# Existing native installer versions also set this marker on worker processes.
# Keep their internal calls headless while user launches default to the GUI.
if [[ ${MERIDIAN_GUI_AUTH:-0} == 1 ]]; then interface=cli; fi
interface_seen=0
while (($#)); do
  case "$1" in
    --install|--check|--update|--repair|--compile|--uninstall)
      ((mode_seen == 0)) || die "Choose only one installation mode"
      mode="${1#--}"; mode_seen=1 ;;
    --gui|--cli)
      selected_interface="${1#--}"
      ((interface_seen == 0)) || [[ $interface == "$selected_interface" ]] || die "Choose either --gui or --cli."
      interface="$selected_interface"; interface_seen=1 ;;
    --prepared) prepared=1 ;;
    --yes|-y) yes=1 ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; die "Unknown argument: $1" ;;
  esac
  shift
done

(( EUID != 0 )) || die "Run as your regular user, not sudo. Privileges are requested only when needed."
[[ -f "$ROOT/Cargo.toml" && -f "$ROOT/tools/install.sh" ]] || die "Run from a complete Meridian source checkout."
[[ "$(uname -s)" == Linux ]] || die "Meridian requires Linux."
((prepared == 0)) || [[ $mode == install || $mode == update || $mode == repair ]] || die "--prepared is only for install, update or repair."
if [[ $interface == gui ]]; then
  ((yes == 0)) || die "--yes requires --cli; confirm graphical operations in the window."
  [[ -n ${WAYLAND_DISPLAY:-} || -n ${DISPLAY:-} ]] || die "No graphical display is available. Use: bash install.sh --cli"
  gui_args=(--source "$ROOT")
  if ((mode_seen)); then gui_args+=(--action "$mode"); fi
  if ((prepared)); then gui_args+=(--prepared); fi
  architecture="$(uname -m)"
  bundled="$ROOT/tools/bin/meridian-installer-linux-$architecture"
  if [[ -f "$bundled" ]]; then
    [[ -x "$bundled" ]] || die "Bundled installer is not executable. Run: chmod +x tools/bin/meridian-installer-linux-$architecture"
    [[ -r "$bundled.sha256" ]] || die "Bundled installer checksum is missing. Obtain a complete checkout/release."
    (cd "$ROOT/tools/bin" && sha256sum --check --status "${bundled##*/}.sha256") || die "Bundled installer checksum failed. Obtain a fresh checkout/release."
    if command -v ldd >/dev/null 2>&1; then
      runtime_report="$(ldd "$bundled" 2>&1)" || die "Bundled installer cannot load on this system: $runtime_report. See tools/bin/README.md."
      if [[ $runtime_report == *"not found"* ]]; then
        die "Bundled installer needs system runtime libraries: $runtime_report. See tools/bin/README.md."
      fi
    fi
    exec "$bundled" "${gui_args[@]}"
  fi
  # Developer-built binaries are usable on architectures without a bundled GUI.
  for binary in "$ROOT/target/release/meridian-installer" "$ROOT/target/debug/meridian-installer"; do
    if [[ -x "$binary" ]]; then exec "$binary" "${gui_args[@]}"; fi
  done
  die "No precompiled GUI for $architecture is included. Obtain a release containing tools/bin/meridian-installer-linux-$architecture, or use: bash install.sh --cli"
fi
if [[ $mode != check ]]; then meridian_privilege_runner || die "Administrator authentication is unavailable."; fi
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
    warn "Prerequisites are missing or outdated. Run: bash install.sh --cli --install"
    return 1
  fi
  note "Preflight passed."
}

if [[ $mode != check ]]; then
  # Serialize builds/installations across this user's source checkouts. No cache
  # directories are created by the read-only --check mode.
  command -v flock >/dev/null 2>&1 || die "Install util-linux (flock) before installing Meridian."
  lock_dir="${XDG_CACHE_HOME:-$HOME/.cache}/meridian"
  mkdir -p "$lock_dir"
  exec {install_lock}>"$lock_dir/install.lock"
  flock -n "$install_lock" || die "Another Meridian installation is already running."
fi

case "$mode" in
  check)
    preflight
    exit 0
    ;;
  uninstall)
    note "Uninstalling Meridian (personal settings and dependency packages are preserved)"
    "${MERIDIAN_PRIVILEGE[@]}" "$ROOT/tools/install.sh" --uninstall
    exit 0
    ;;
esac


if ((prepared)); then
  note "Checking prepared release integrity (no compilation)"
  "$ROOT/tools/install.sh" --check-build
  note "Installing the prepared release transactionally"
  "${MERIDIAN_PRIVILEGE[@]}" "$ROOT/tools/install.sh"
  note "Complete. Sign out and back into Meridian to load this version."
  exit 0
fi

# Source upgrades are explicit: never fetch or reset a user's working tree.
if [[ "$mode" == update ]]; then
  note "Updating from the CURRENT checkout; run git pull yourself for newer source."
fi

if (( ! yes )); then
  printf '\nThis will install or update dependency packages using %s\n' "$(
    case "$DISTRO" in debian) echo APT;; fedora) echo DNF;; arch) echo pacman;; esac
  )"
  if [[ $mode == compile ]]; then
    printf 'and compile a release only; Meridian system files will not be installed.\n'
  else
    printf 'and compile and install Meridian under /usr/local.\n'
  fi
  printf 'On Arch, the package step may upgrade the system.\n'
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
if [[ $mode == compile ]]; then
  note "Compilation complete. Install this prepared release with: bash install.sh --cli --prepared"
  exit 0
fi
note "Checking release integrity"
"$ROOT/tools/install.sh" --check-build
note "Installing the release transactionally (administrator authentication)"
"${MERIDIAN_PRIVILEGE[@]}" "$ROOT/tools/install.sh"
note "Installation complete"
printf 'Select Meridian from your existing login screen after signing out.\n'
printf 'To update from newer code: git pull, then bash install.sh --cli --update\n'
printf 'To remove Meridian: bash install.sh --cli --uninstall\n'
