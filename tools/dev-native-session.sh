#!/usr/bin/env bash
# Start a separate labwc session with only the Swift/C++ prototype as its shell.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
command -v labwc >/dev/null || { echo 'labwc is required' >&2; exit 1; }
export MERIDIAN_NATIVE_RUNNER="$root/tools/dev-native.sh"
export XDG_CURRENT_DESKTOP=MarsOS
export XDG_SESSION_DESKTOP=marsos-native
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
printf '%s\n' 'Starting native prototype. Super+Q exits this test compositor.'
labwc -C "$configuration" -s 'exec "$MERIDIAN_NATIVE_RUNNER" --session'
