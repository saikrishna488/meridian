#!/usr/bin/env bash
# Run with a built UI. All desktop resources, configuration and D-Bus are private.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
for tool in labwc dbus-run-session timeout cargo cc pkg-config wayland-scanner kitty; do command -v "$tool" >/dev/null || { echo "Missing integration dependency: $tool" >&2; exit 1; }; done
fixture=$(mktemp -d)
cleanup() { python3 - "$fixture" <<'PY'
import shutil, sys
shutil.rmtree(sys.argv[1])
PY
}
trap cleanup EXIT
mkdir -p "$fixture"/{runtime,config,data,cache,home,labwc}
mkdir -p "$fixture/config/kitty"
printf 'confirm_os_window_close 0\n' > "$fixture/config/kitty/kitty.conf"
chmod 700 "$fixture/runtime"
printf '<labwc_config><core><decoration>server</decoration></core></labwc_config>\n' > "$fixture/labwc/rc.xml"
cargo build --manifest-path "$ROOT/Cargo.toml" -p meridian-shell --features integration-tests --bin meridian-shell --bin meridian-smoke
protocol=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -path '*/wayland-protocols-wlr-*/wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml' -print -quit)
[ -n "$protocol" ] || { echo "Missing wlroots virtual pointer protocol" >&2; exit 1; }
wayland-scanner client-header "$protocol" "$fixture/virtual-pointer.h"
wayland-scanner private-code "$protocol" "$fixture/virtual-pointer.c"
cc -I"$fixture" "$ROOT/tools/tests/wayland-pointer.c" "$fixture/virtual-pointer.c" -o "$fixture/pointer" $(pkg-config --cflags --libs wayland-client)
cc "$ROOT/tools/tests/wayland-client.c" -o "$fixture/client" $(pkg-config --cflags --libs gtk4)
cat > "$fixture/run" <<'RUN'
#!/bin/sh
export MERIDIAN_TEST_COMPOSITOR_PID="$PPID"
if [ -n "${MERIDIAN_TEST_SCALE:-}" ]; then
    "${MERIDIAN_TEST_WLR_RANDR:-wlr-randr}" --output HEADLESS-1 --scale "$MERIDIAN_TEST_SCALE" --output HEADLESS-2 --scale "$MERIDIAN_TEST_SCALE" || exit 1
fi
"$MERIDIAN_TEST_ROOT/target/debug/meridian-smoke"
status=$?
if [ "$status" -eq 0 ]; then python3 "$MERIDIAN_TEST_ROOT/tools/tests/shell-process.py"; status=$?; fi
printf "%s\n" "$status" > "$MERIDIAN_TEST_RESULT"
exit "$status"
RUN
chmod +x "$fixture/run"
export MERIDIAN_TEST_POINTER="$fixture/pointer"
export MERIDIAN_TEST_CLIENT="$fixture/client"
export MERIDIAN_TEST_RESULT="$fixture/result"
export MERIDIAN_TEST_ROOT="$ROOT" MERIDIAN_NESTED_TEST=1 MERIDIAN_DEV=1
export MERIDIAN_UI_DIR="${MERIDIAN_TEST_UI_DIR:-$ROOT/ui/dist}" MERIDIAN_ASSETS_DIR="$ROOT/assets"
export XDG_RUNTIME_DIR="$fixture/runtime" XDG_CONFIG_HOME="$fixture/config" XDG_DATA_HOME="$fixture/data" XDG_CACHE_HOME="$fixture/cache"
export DBUS_SYSTEM_BUS_ADDRESS="unix:path=$fixture/no-system-bus"
export GSK_RENDERER=cairo
export G_DEBUG=fatal-criticals
export WLR_HEADLESS_OUTPUTS=2
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
printf 'XDG_DESKTOP_DIR="%s/home/Desktop"\n' "$fixture" > "$fixture/config/user-dirs.dirs"
export SHELL=/bin/sh HISTFILE="$fixture/home/history"
if [ "$(id -u)" -eq 0 ]; then
    # WebKit's sandbox cannot run as root in the disposable CI container.
    export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
fi
unset WAYLAND_DISPLAY DISPLAY
# HOME remains untouched for Cargo's toolchain. XDG paths isolate Meridian state.
soak_seconds=${MERIDIAN_SOAK_SECONDS:-0}
case "$soak_seconds" in ''|*[!0-9]*) echo 'MERIDIAN_SOAK_SECONDS must be an integer' >&2; exit 1;; esac
[ "${#soak_seconds}" -le 5 ] && [ "$soak_seconds" -le 86400 ] || { echo 'Soak is limited to 86400 seconds' >&2; exit 1; }
timeout --kill-after=10s "$((180 + soak_seconds))s" dbus-run-session -- labwc -C "$fixture/labwc" -S "$fixture/run"

[ -f "$fixture/result" ] && [ "$(cat "$fixture/result")" = 0 ] || { echo "Nested desktop test failed or exited without a result" >&2; exit 1; }
