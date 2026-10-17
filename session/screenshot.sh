#!/bin/sh
# Embedded in the native shell, so installed and development sessions match.
set -eu
umask 077
notify() { notify-send --app-name=Meridian "$1" "$2" 2>/dev/null || true; }
fail() { notify "Screenshot failed" "$1"; printf '%s\n' "$1" >&2; exit 1; }
command -v grim >/dev/null 2>&1 || fail "Install grim to take screenshots."
case "${1:-full}" in
    full) geometry= ;;
    region)
        command -v slurp >/dev/null 2>&1 || fail "Install slurp to select a screenshot area."
        geometry=$(slurp) || exit 0
        [ -n "$geometry" ] || exit 0
        ;;
    *) fail "Unknown screenshot mode." ;;
esac
pictures=$(xdg-user-dir PICTURES 2>/dev/null) || pictures="$HOME/Pictures"
[ -n "$pictures" ] || pictures="$HOME/Pictures"
directory="$pictures/Screenshots"
mkdir -p "$directory" || fail "Cannot create the Screenshots folder."
image=$(mktemp "$directory/Screenshot-$(date +%Y-%m-%d-%H%M%S)-XXXXXX.png") || fail "Cannot create the screenshot file."
trap 'rm -f "$image"' EXIT HUP INT TERM
if [ -n "$geometry" ]; then
    grim -g "$geometry" "$image" || fail "Could not capture the selected area."
else
    grim "$image" || fail "Could not capture the desktop."
fi
trap - EXIT HUP INT TERM
notify "Screenshot saved" "$image"
