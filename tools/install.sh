#!/usr/bin/env bash
# Install Meridian on this machine (Fedora). Run from the repository, as root:
#
#   sudo tools/install.sh                   install, and add "Meridian" to the
#                                           current login screen's session list
#   sudo tools/install.sh --full-codecs     also enable RPM Fusion Free codecs
#   sudo tools/install.sh --try-login-screen
#                                           start Meridian's login screen on VT 7
#                                           alongside the current one, to test it
#   sudo tools/install.sh --enable-login-screen
#                                           make Meridian's login screen the
#                                           default (takes effect after reboot)
#   sudo tools/install.sh --disable-login-screen
#                                           go back to the previous login screen
#   sudo tools/install.sh --uninstall       remove everything this script added
#
# Build first (as your user): tools/build-release.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tools/codecs.sh"
PREFIX=/usr/local
SHARE="$PREFIX/share/meridian"
SESSION_FILE=/usr/share/wayland-sessions/meridian.desktop
GREETD_CONFIG=/etc/greetd/config.toml
GREETD_BACKUP=/etc/greetd/config.toml.before-meridian
TRIAL_UNIT=meridian-login-trial
# The login manager Meridian's greeter replaces (Fedora KDE 44 default).
PREVIOUS_DM_FILE=/etc/greetd/.meridian-previous-dm

PACKAGES=(
    webkitgtk6.0 gtk4-layer-shell labwc xorg-x11-server-Xwayland
    wireplumber rsms-inter-fonts gsettings-desktop-schemas NetworkManager NetworkManager-wifi bluez ddcutil
    greetd greetd-selinux
)

die() { echo "error: $*" >&2; exit 1; }
step() { printf '\033[1m==> %s\033[0m\n' "$*"; }

[ "$(id -u)" = 0 ] || die "run as root: sudo $0 $*"

greeter_command() {
    echo "labwc -C $SHARE/greeter/labwc -S $PREFIX/bin/meridian-greeter"
}

install_files() {
    for f in target/release/meridian-shell target/release/meridian-greeter ui/dist/surfaces/panel/index.html \
        ui/dist/surfaces/locker/index.html ui/dist/surfaces/locker/locker.js \
        ui/dist/surfaces/finder/finder.js ui/dist/surfaces/terminal/terminal.js \
        ui/dist/surfaces/settings/settings.js ui/dist/vendor/xterm.css ui/dist/vendor/LICENSE; do
        [ -e "$ROOT/$f" ] || die "$f is missing; build first with tools/build-release.sh (as your user)"
    done

    step "Installing runtime packages"
    dnf install -y --setopt=install_weak_deps=False "${PACKAGES[@]}"
    step "Installing multimedia codecs"
    media_packages_for_host
    dnf install -y --setopt=install_weak_deps=False "${MEDIA_SELECTED[@]}"

    step "Installing Meridian to $PREFIX"
    install -Dm755 "$ROOT/target/release/meridian-shell" "$PREFIX/bin/meridian-shell"
    install -Dm755 "$ROOT/target/release/meridian-greeter" "$PREFIX/bin/meridian-greeter"
    sed "s|@SHARE_DIR@|$SHARE|" "$ROOT/session/meridian-session" > "$PREFIX/bin/meridian-session"
    chmod 755 "$PREFIX/bin/meridian-session"

    rm -rf "$SHARE"
    install -d "$SHARE"
    cp -r "$ROOT/ui/dist" "$SHARE/ui"
    find "$SHARE/ui" -name '*.map' -delete
    cp -r "$ROOT/assets" "$SHARE/assets"
    install -m644 "$ROOT/LICENSE" "$SHARE/LICENSE"
    cp -r "$ROOT/session/labwc" "$SHARE/labwc"
    install -d "$SHARE/greeter"
    cp -r "$ROOT/session/greeter/labwc" "$SHARE/greeter/labwc"
    install -d "$PREFIX/share/themes/Meridian"
    cp -r "$ROOT/assets/themes/Meridian/." "$PREFIX/share/themes/Meridian/"
    install -m644 "$ROOT/LICENSE" "$PREFIX/share/themes/Meridian/LICENSE"
    chmod -R a+rX "$SHARE" "$PREFIX/share/themes/Meridian"

    step "Adding the Meridian session to the login screen"
    sed "s|^Exec=.*|Exec=$PREFIX/bin/meridian-session|" "$ROOT/session/meridian.desktop" > "$SESSION_FILE"
    chmod 644 "$SESSION_FILE"

    step "Installing the lock screen's PAM service"
    install -Dm644 "$ROOT/session/meridian-lock.pam" /etc/pam.d/meridian-lock

    if command -v restorecon >/dev/null; then
        restorecon -R "$PREFIX/bin/meridian-"* "$SHARE" "$SESSION_FILE" /etc/pam.d/meridian-lock
    fi
    echo "Installed. \"Meridian\" now appears in the login screen's session list."
}

write_greetd_config() {
    local vt="$1" path="$2" switch="${3:-true}"
    cat > "$path" <<EOF
# Written by Meridian's tools/install.sh.
[terminal]
vt = $vt
switch = $switch

[default_session]
command = "$(greeter_command)"
user = "greetd"
EOF
}

current_dm() {
    local unit
    unit="$(readlink -f /etc/systemd/system/display-manager.service 2>/dev/null || true)"
    basename "${unit%.service}" 2>/dev/null || true
}

try_login_screen() {
    [ -x "$PREFIX/bin/meridian-greeter" ] || die "install first: sudo $0"
    local config=/run/meridian-login-trial.toml
    write_greetd_config 7 "$config" false
    systemctl stop "$TRIAL_UNIT" 2>/dev/null || true
    step "Starting Meridian's login screen on VT 7"
    systemd-run --unit="$TRIAL_UNIT" --description="Meridian login screen (trial)" \
        /usr/sbin/greetd --config "$config"
    cat <<EOF
The trial login screen is running on virtual terminal 7.
  - Switch to it:      Ctrl+Alt+F7 (logging in there starts a second session)
  - Come back here:    Ctrl+Alt+F1 or Ctrl+Alt+F2 (whichever this session is on)
  - Stop the trial:    sudo systemctl stop $TRIAL_UNIT
EOF
}

enable_login_screen() {
    [ -x "$PREFIX/bin/meridian-greeter" ] || die "install first: sudo $0"
    local previous
    previous="$(current_dm)"
    [ "$previous" = greetd ] && { echo "Meridian's login screen is already enabled."; return; }

    step "Configuring greetd"
    if [ -f "$GREETD_CONFIG" ] && [ ! -f "$GREETD_BACKUP" ]; then
        cp -a "$GREETD_CONFIG" "$GREETD_BACKUP"
    fi
    write_greetd_config 1 "$GREETD_CONFIG"
    echo "$previous" > "$PREVIOUS_DM_FILE"

    step "Switching the login screen: $previous → greetd (Meridian)"
    [ -n "$previous" ] && systemctl disable "$previous"
    systemctl enable greetd
    cat <<EOF
Done. Meridian's login screen will appear after the next reboot.
To go back:   sudo $0 --disable-login-screen
If it ever fails to appear: press Ctrl+Alt+F3, log in as text, then run
              sudo systemctl disable greetd && sudo systemctl enable ${previous:-plasmalogin} && sudo reboot
EOF
}

disable_login_screen() {
    local previous=plasmalogin
    [ -f "$PREVIOUS_DM_FILE" ] && previous="$(cat "$PREVIOUS_DM_FILE")"
    if [ "$(current_dm)" = greetd ]; then
        step "Switching the login screen back: greetd → $previous"
        systemctl disable greetd
        systemctl enable "$previous"
        echo "The previous login screen returns after the next reboot."
    fi
    if [ -f "$GREETD_BACKUP" ]; then
        mv -f "$GREETD_BACKUP" "$GREETD_CONFIG"
    fi
    rm -f "$PREVIOUS_DM_FILE"
}

uninstall() {
    systemctl stop "$TRIAL_UNIT" 2>/dev/null || true
    disable_login_screen
    step "Removing Meridian files"
    rm -f "$PREFIX/bin/meridian-shell" "$PREFIX/bin/meridian-greeter" "$PREFIX/bin/meridian-session" "$SESSION_FILE" \
        /etc/pam.d/meridian-lock
    rm -rf "$SHARE" "$PREFIX/share/themes/Meridian"
    echo "Removed. (Runtime packages were left installed: ${PACKAGES[*]})"
}

case "${1:-}" in
    "") install_files ;;
    --full-codecs) install_files; bash "$ROOT/tools/install-codecs.sh" --full ;;
    --try-login-screen) try_login_screen ;;
    --enable-login-screen) enable_login_screen ;;
    --disable-login-screen) disable_login_screen ;;
    --uninstall) uninstall ;;
    *) die "unknown option: $1 (see the top of $0)" ;;
esac
