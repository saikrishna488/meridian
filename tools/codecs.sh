# Compatibility helper: load multimedia packages from the host profile.
source "$(dirname "${BASH_SOURCE[0]}")/distro.sh"
meridian_load_packages
media_packages_for_host() { MEDIA_SELECTED=("${MEDIA_PACKAGES[@]}"); }
