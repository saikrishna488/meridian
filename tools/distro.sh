# Distribution profiles shared by setup, build and installation scripts.
# Sourced by bash. Select by os-release, never by whichever manager is on PATH.
meridian_detect_distro() {
    local ID= ID_LIKE= candidate
    DISTRO=
    source "${MERIDIAN_OS_RELEASE:-/etc/os-release}"
    for candidate in "$ID" $ID_LIKE; do
        case "$candidate" in
            debian|ubuntu) DISTRO=debian; return ;;
            fedora) DISTRO=fedora; return ;;
            arch) DISTRO=arch; return ;;
        esac
    done
    echo "Unsupported distribution: ${ID:-unknown}. Supported families: Debian, Fedora, Arch." >&2
    return 1
}

meridian_load_packages() {
    meridian_detect_distro || return
    local common=(swayidle wlopm grim slurp labwc wireplumber gvfs
        gnome-disk-utility udisks2 util-linux gsettings-desktop-schemas bluez ddcutil
        greetd kmod accountsservice openssl curl ca-certificates xdg-user-dirs kitty
        nautilus gnome-calendar gnome-clocks gnome-software flatpak)
    case "$DISTRO" in
        debian)
            source "$(dirname "${BASH_SOURCE[0]}")/debian.sh"
            PIPEWIRE_BUILD_PACKAGES=(pkg-config libpipewire-0.3-dev)
            MEDIA_PACKAGES=(gstreamer1.0-plugins-base gstreamer1.0-plugins-good
                gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav ffmpeg)
            ;;
        fedora)
            BUILD_PACKAGES=(gcc gcc-c++ make pkgconf-pkg-config rust cargo rustfmt clippy
                nodejs npm gtk4-devel webkitgtk6.0-devel gtk4-layer-shell-devel pam-devel clang-devel pipewire-devel)
            PIPEWIRE_BUILD_PACKAGES=(pkgconf-pkg-config pipewire-devel)
            RUNTIME_PACKAGES=("${common[@]}" glib2 libnotify webkitgtk6.0 gtk4-layer-shell
                xorg-x11-server-Xwayland greetd-selinux pipewire-utils pipewire-pulseaudio pipewire-libs
                rsms-inter-fonts google-noto-sans-fonts dejavu-sans-fonts liberation-fonts
                cups-client NetworkManager pam polkit mate-polkit dbus-daemon)
            MEDIA_PACKAGES=(gstreamer1-plugins-base gstreamer1-plugins-good
                gstreamer1-plugins-bad-free ffmpeg-free)
            ;;
        arch)
            BUILD_PACKAGES=(base-devel rust nodejs npm gtk4 webkitgtk-6.0 gtk4-layer-shell pam clang pipewire)
            PIPEWIRE_BUILD_PACKAGES=(pkgconf pipewire)
            RUNTIME_PACKAGES=("${common[@]}" glib2 libnotify webkitgtk-6.0 gtk4-layer-shell
                xorg-xwayland pipewire pipewire-audio pipewire-pulse inter-font noto-fonts
                ttf-dejavu ttf-liberation cups networkmanager pam polkit polkit-gnome dbus)
            MEDIA_PACKAGES=(gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav ffmpeg)
            ;;
    esac
}

# Optional runner arguments, e.g. meridian_install_packages sudo -- packages...
meridian_install_packages() {
    local runner=()
    while [ "$#" -gt 0 ] && [ "$1" != -- ]; do runner+=("$1"); shift; done
    [ "${1:-}" = -- ] || { echo 'Missing package separator' >&2; return 1; }
    shift
    case "$DISTRO" in
        debian) "${runner[@]}" apt-get update &&
            "${runner[@]}" apt-get install -y --no-install-recommends "$@" ;;
        fedora) "${runner[@]}" dnf install -y "$@" ;;
        # A full upgrade avoids unsupported partial upgrades on Arch.
        arch) "${runner[@]}" pacman -Syu --needed --noconfirm "$@" ;;
        *) echo 'Load a distribution profile first' >&2; return 1 ;;
    esac
}

meridian_write_lock_pam() {
    case "$DISTRO" in
        debian) cat "$(dirname "${BASH_SOURCE[0]}")/../session/meridian-lock.pam" ;;
        fedora) printf '#%%PAM-1.0\nauth include system-auth\naccount include system-auth\n' ;;
        arch) printf '#%%PAM-1.0\nauth include system-local-login\naccount include system-local-login\n' ;;
        *) return 1 ;;
    esac
}
