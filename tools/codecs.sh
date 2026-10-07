# Shared Fedora multimedia package list, sourced by install/dev scripts.
# Keep explicit package names: no wildcard pulling in devel/debug packages.
MEDIA_PACKAGES=(
    gstreamer1-plugins-base gstreamer1-plugins-good
    gstreamer1-plugins-good-extras gstreamer1-plugins-bad-free
    gstreamer1-plugins-ugly-free gstreamer1-plugin-libav
    openh264 gstreamer1-plugin-openh264 mozilla-openh264
)

# Existing full FFmpeg / ugly plugin installations already provide these
# features. Avoid replacing them with Fedora's reduced variants.
media_packages_for_host() {
    MEDIA_SELECTED=()
    local package
    for package in "${MEDIA_PACKAGES[@]}"; do
        if [ "$package" = gstreamer1-plugins-ugly-free ] && rpm -q gstreamer1-plugins-ugly >/dev/null 2>&1; then
            continue
        fi
        if [ "$package" = gstreamer1-plugin-libav ] && rpm -q gstreamer1-libav >/dev/null 2>&1; then
            continue
        fi
        MEDIA_SELECTED+=("$package")
    done
    if ! rpm -q ffmpeg >/dev/null 2>&1; then MEDIA_SELECTED+=(ffmpeg-free); fi
}
