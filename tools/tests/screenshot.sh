#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/bin" "$scratch/Pictures with spaces"
export SCREENSHOT_TEST_ROOT="$scratch"
export PATH="$scratch/bin:$PATH"
cat > "$scratch/bin/xdg-user-dir" <<'MOCK'
#!/bin/sh
printf '%s\n' "$SCREENSHOT_TEST_ROOT/Pictures with spaces"
MOCK
cat > "$scratch/bin/notify-send" <<'MOCK'
#!/bin/sh
printf '%s\n' "$@" >> "$SCREENSHOT_TEST_ROOT/notifications"
MOCK
cat > "$scratch/bin/slurp" <<'MOCK'
#!/bin/sh
[ "${CANCEL:-0}" = 0 ] || exit 1
printf '%s\n' '10,20 300x200'
MOCK
cat > "$scratch/bin/grim" <<'MOCK'
#!/bin/sh
printf '%s\n' "$@" >> "$SCREENSHOT_TEST_ROOT/captures"
[ "${FAIL_CAPTURE:-0}" = 0 ] || exit 1
for destination do :; done
printf 'mock PNG' > "$destination"
MOCK
chmod +x "$scratch/bin/"*
sh "$root/session/screenshot.sh" full
sh "$root/session/screenshot.sh" region
count=$(find "$scratch/Pictures with spaces" -name '*.png' | wc -l)
[ "$count" -eq 2 ]
grep -Fqx '10,20 300x200' "$scratch/captures"
CANCEL=1 sh "$root/session/screenshot.sh" region
[ "$(find "$scratch/Pictures with spaces" -name '*.png' | wc -l)" -eq 2 ]
if FAIL_CAPTURE=1 sh "$root/session/screenshot.sh" full 2>/dev/null; then
    echo 'Expected capture failure' >&2; exit 1
fi
[ "$(find "$scratch/Pictures with spaces" -name '*.png' | wc -l)" -eq 2 ]
grep -Fq 'Screenshot failed' "$scratch/notifications"
echo 'Screenshot capture, region, cancellation, failure cleanup, and spaced paths passed.'
