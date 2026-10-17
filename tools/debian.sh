# Debian-family package names shared by development and installation scripts.
require_debian() {
    local ID= ID_LIKE=
    source /etc/os-release
    case " $ID $ID_LIKE " in
        *" debian "*|*" ubuntu "*) ;;
        *) echo "Meridian requires Debian or a Debian-derived distribution." >&2; return 1 ;;
    esac
}
BUILD_PACKAGES=(build-essential pkg-config rustc cargo rustfmt rust-clippy nodejs npm
    libgtk-4-dev libwebkitgtk-6.0-dev libgtk4-layer-shell-dev libpam0g-dev libclang-dev)
RUNTIME_PACKAGES=(libwebkitgtk-6.0-4 libgtk4-layer-shell0 labwc xwayland
    wireplumber fonts-inter gsettings-desktop-schemas network-manager bluez ddcutil
    greetd libpam-modules libpam-runtime polkitd pkexec xdg-user-dirs dbus-user-session)
