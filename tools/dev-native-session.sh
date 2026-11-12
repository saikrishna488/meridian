#!/usr/bin/env bash
# Start a separate labwc session with only the Swift/C++ prototype as its shell.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
command -v labwc >/dev/null || { echo 'labwc is required' >&2; exit 1; }
export MERIDIAN_NATIVE_RUNNER="${MERIDIAN_NATIVE_RUNNER:-$root/tools/dev-native.sh}"
# Fail before opening a compositor if the executable or runtime is missing.
"$MERIDIAN_NATIVE_RUNNER" --check
if [[ ${MERIDIAN_NATIVE_PRIVATE_BUS:-0} != 1 ]]; then
    command -v dbus-run-session >/dev/null || { echo 'dbus-run-session is required' >&2; exit 1; }
    # A second GApplication on the parent's bus can activate an existing window
    # on the wrong display and exit, leaving the nested compositor empty.
    exec env MERIDIAN_NATIVE_PRIVATE_BUS=1 dbus-run-session -- bash "$0" "$@"
fi
export XDG_CURRENT_DESKTOP=MarsOS
export XDG_SESSION_DESKTOP=marsos-native
export XDG_SESSION_TYPE=wayland
configuration=$(mktemp -d "${TMPDIR:-/tmp}/marsos-native-session.XXXXXX")
trap 'rm -rf -- "$configuration"' EXIT
cat > "$configuration/rc.xml" <<'XML'
<?xml version="1.0"?>
<labwc_config>
  <keyboard>
    <keybind key="W-q"><action name="Exit"/></keybind>
    <keybind key="A-F4"><action name="Close"/></keybind>
    <keybind key="A-Tab"><action name="NextWindow"/></keybind>
  </keyboard>
  <windowRules><windowRule identifier="org.meridian.NativeApplications"><action name="Maximize"/></windowRule></windowRules>
</labwc_config>
XML
log="${MERIDIAN_NATIVE_SESSION_LOG:-$root/target/native-session.log}"
mkdir -p "$(dirname "$log")"
# labwc parses this into argv; `exec` is a shell builtin, not an executable.
printf -v startup '%q --session' "$MERIDIAN_NATIVE_RUNNER"
printf 'Starting native prototype. Super+Q exits. Log: %s\n' "$log"
# -S terminates the compositor when the shell exits, including startup failure.
labwc -C "$configuration" -S "$startup" 2>&1 | tee "$log"
