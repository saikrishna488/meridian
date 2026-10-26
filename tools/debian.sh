Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
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
    libgtk-4-dev libwebkitgtk-6.0-dev libgtk4-layer-shell-dev libpam0g-dev libclang-dev libpipewire-0.3-dev)
RUNTIME_PACKAGES=(swayidle wlopm libglib2.0-bin grim slurp libnotify-bin libwebkitgtk-6.0-4 libgtk4-layer-shell0 labwc xwayland
    wireplumber pipewire-bin pipewire-pulse libspa-0.2-bluetooth gvfs fonts-inter fonts-noto-core fonts-dejavu-core fonts-liberation gnome-disk-utility udisks2 util-linux cups-client gsettings-desktop-schemas network-manager bluez ddcutil
    greetd libpam-modules libpam-runtime polkitd pkexec policykit-1-gnome kmod accountsservice openssl curl ca-certificates xdg-user-dirs dbus-user-session kitty nautilus
    gnome-calendar gnome-clocks gnome-software gnome-software-plugin-flatpak flatpak)
