# Multimedia codecs from the distribution's Debian repositories.
MEDIA_PACKAGES=(gstreamer1.0-plugins-base gstreamer1.0-plugins-good
    gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav ffmpeg)
media_packages_for_host() { MEDIA_SELECTED=("${MEDIA_PACKAGES[@]}"); }
