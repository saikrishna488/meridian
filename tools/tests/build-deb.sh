#!/usr/bin/env bash
# Inspect a real Debian archive without installing anything on the host.
set -euo pipefail
TOOLS="$(cd "$(dirname "$0")/.." && pwd)"
source "$TOOLS/release-manifest.sh"
source "$TOOLS/debian.sh"
source "$TOOLS/codecs.sh"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
ROOT="$fixture/repo"
mkdir -p "$ROOT"/{protocol,services,shell,ui/{src,scripts,dist/surfaces/panel},assets/themes/Meridian/openbox-3,session/{labwc,greeter/labwc},tools,target/release}
cp "$TOOLS"/{build-deb.sh,release-manifest.sh,debian.sh,codecs.sh} "$ROOT/tools/"
printf '[workspace.package]\nversion = "0.1.0"\n' > "$ROOT/Cargo.toml"
printf '#!/bin/sh\nshare=@SHARE_DIR@\n' > "$ROOT/session/meridian-session"
printf '[Desktop Entry]\nExec=meridian-session\n' > "$ROOT/session/meridian.desktop"
for file in Cargo.lock rustfmt.toml LICENSE ui/package.json ui/package-lock.json ui/tsconfig.json \
    ui/dist/main.js ui/dist/main.js.map ui/dist/surfaces/panel/index.html \
    target/release/meridian-shell target/release/meridian-greeter session/shell-supervisor \
    session/labwc/autostart session/meridian-lock.pam session/greeter/labwc/rc.xml assets/themes/Meridian/openbox-3/themerc; do
    printf 'fixture\n' > "$ROOT/$file"
done
release_manifest > "$ROOT/target/release/meridian-build.sha256"
package="$fixture/package with spaces.deb"
# Restrictive builder permissions must not make installed assets unreadable.
(umask 077; bash "$ROOT/tools/build-deb.sh" --no-build --output "$package")
[ "$(dpkg-deb -f "$package" Package)" = meridian-desktop ]
[ "$(dpkg-deb -f "$package" Version)" = 0.1.0 ]
[ "$(dpkg-deb -f "$package" Architecture)" = "$(dpkg --print-architecture)" ]
depends="$(dpkg-deb -f "$package" Depends)"
for dependency in "${RUNTIME_PACKAGES[@]}" "${MEDIA_PACKAGES[@]}"; do
    case ", $depends, " in
        *", $dependency, "*|*", $dependency ("*) ;;
        *) echo "Missing dependency: $dependency" >&2; exit 1 ;;
    esac
done
for dependency in 'libgtk-4-1 (>= 4.18)' 'libwebkitgtk-6.0-4 (>= 2.52)' 'libgtk4-layer-shell0 (>= 1.2)'; do
    [[ "$depends" == *"$dependency"* ]]
done
dpkg-deb -x "$package" "$fixture/unpacked"
dpkg-deb -e "$package" "$fixture/control"
share="$fixture/unpacked/usr/local/share/meridian"
[ -x "$fixture/unpacked/usr/local/bin/meridian-shell" ]
[ -x "$fixture/unpacked/usr/local/bin/meridian-greeter" ]
[ -x "$fixture/unpacked/usr/local/bin/meridian-session" ]
[ -x "$share/shell-supervisor" ]
[ -f "$share/greeter/labwc/rc.xml" ]
[ -f "$share/labwc/autostart" ]
[ -f "$fixture/unpacked/etc/pam.d/meridian-lock" ]
[ -f "$share/ui/main.js" ]
[ ! -e "$share/ui/main.js.map" ]
[ -f "$fixture/unpacked/usr/local/share/themes/Meridian/LICENSE" ]
rg -q '^share=/usr/local/share/meridian$' "$fixture/unpacked/usr/local/bin/meridian-session"
rg -q '^Exec=/usr/local/bin/meridian-session$' "$fixture/unpacked/usr/share/wayland-sessions/meridian.desktop"
[ -z "$(find "$share" -type f ! -perm -004 -print)" ]
[ -z "$(find "$share" -type d ! -perm -005 -print)" ]
sh -n "$fixture/control/postinst"
bash "$ROOT/tools/build-deb.sh" --help >/dev/null
if bash "$ROOT/tools/build-deb.sh" --invalid >/dev/null 2>&1; then exit 1; fi
if bash "$ROOT/tools/build-deb.sh" --no-build --output "$ROOT/Cargo.toml" >/dev/null 2>&1; then
    echo 'Allowed packaging to overwrite source' >&2; exit 1
fi
printf 'changed\n' >> "$ROOT/ui/dist/main.js"
if bash "$ROOT/tools/build-deb.sh" --no-build --output "$fixture/stale.deb" >/dev/null 2>&1; then
    echo 'Packaged stale release' >&2; exit 1
fi
echo 'Debian package contents, dependencies, permissions, and release verification passed.'
