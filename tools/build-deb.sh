#!/usr/bin/env bash
# Build one installable Debian package from the verified Meridian release.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\([0-9][^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
ARCH="$(dpkg --print-architecture)"
OUTPUT="$ROOT/target/meridian_${VERSION}_${ARCH}.deb"
BUILD=true

while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-build) BUILD=false; shift ;;
        --output) [ "$#" -ge 2 ] || { echo "Missing output path." >&2; exit 2; }; OUTPUT="$2"; shift 2 ;;
        --help|-h) echo "Usage: tools/build-deb.sh [--no-build] [--output FILE]"; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

command -v dpkg-deb >/dev/null 2>&1 || { echo "error: dpkg-deb is required" >&2; exit 1; }
OUTPUT="$(realpath -m "$OUTPUT")"
# Packaging must not overwrite source inputs or compiled release artifacts.
case "$OUTPUT" in
    "$ROOT/target/meridian_${VERSION}_${ARCH}.deb") ;;
    "$ROOT"/*) echo 'Use the default output or a path outside the checkout.' >&2; exit 2 ;;
esac

source "$ROOT/tools/debian.sh"
source "$ROOT/tools/codecs.sh"
dependencies=("libgtk-4-1 (>= 4.18)")
for package in "${RUNTIME_PACKAGES[@]}" "${MEDIA_PACKAGES[@]}"; do
    case "$package" in
        libwebkitgtk-6.0-4) dependencies+=("$package (>= 2.52)") ;;
        libgtk4-layer-shell0) dependencies+=("$package (>= 1.2)") ;;
        *) dependencies+=("$package") ;;
    esac
done
depends="$(printf '%s, ' "${dependencies[@]}")"
depends="${depends%, }"

if "$BUILD"; then
    bash "$ROOT/tools/build-release.sh"
fi

source "$ROOT/tools/release-manifest.sh"
verify_release

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
pkg="$stage/package"
share="$pkg/usr/local/share/meridian"

install -d "$pkg/DEBIAN" "$pkg/usr/local/bin" "$pkg/usr/share/wayland-sessions" "$pkg/usr/local/share/themes/Meridian"
install -Dm755 "$ROOT/target/release/meridian-shell" "$pkg/usr/local/bin/meridian-shell"
install -Dm755 "$ROOT/target/release/meridian-greeter" "$pkg/usr/local/bin/meridian-greeter"
sed "s|@SHARE_DIR@|/usr/local/share/meridian|" "$ROOT/session/meridian-session" > "$pkg/usr/local/bin/meridian-session"
chmod 755 "$pkg/usr/local/bin/meridian-session"
sed 's|^Exec=.*|Exec=/usr/local/bin/meridian-session|' "$ROOT/session/meridian.desktop" > "$pkg/usr/share/wayland-sessions/meridian.desktop"
chmod 644 "$pkg/usr/share/wayland-sessions/meridian.desktop"

install -Dm755 "$ROOT/session/shell-supervisor" "$share/shell-supervisor"
cp -a "$ROOT/ui/dist" "$share/ui"
cp -a "$ROOT/assets" "$share/assets"
cp -a "$ROOT/session/labwc" "$share/labwc"
install -d "$share/greeter"
cp -a "$ROOT/session/greeter/labwc" "$share/greeter/labwc"
install -Dm644 "$ROOT/LICENSE" "$share/LICENSE"
cp -a "$ROOT/assets/themes/Meridian/." "$pkg/usr/local/share/themes/Meridian/"
install -m644 "$ROOT/LICENSE" "$pkg/usr/local/share/themes/Meridian/LICENSE"
find "$share/ui" -name '*.map' -delete
chmod -R a+rX "$share" "$pkg/usr/local/share/themes/Meridian"

cat > "$pkg/DEBIAN/control" <<EOF
Package: meridian-desktop
Version: $VERSION
Section: x11
Priority: optional
Architecture: $ARCH
Depends: $depends
Maintainer: Meridian contributors
Description: Meridian desktop shell
 A focused Wayland desktop shell built with GTK and WebKit.
EOF

cat > "$pkg/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/local/share/applications >/dev/null 2>&1 || true
fi
if command -v flatpak >/dev/null 2>&1; then
    flatpak remote-add --system --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo >/dev/null 2>&1 || true
fi
echo "Meridian installed. Select the Meridian session from your login screen."
exit 0
EOF
chmod 755 "$pkg/DEBIAN/postinst"

mkdir -p "$(dirname "$OUTPUT")"
dpkg-deb --build --root-owner-group "$pkg" "$OUTPUT" >/dev/null
echo "Created $OUTPUT"
echo "Install: sudo apt install '$OUTPUT'"
