#!/usr/bin/env bash
# Exercise lock confirmation and idle commands without locking the developer's session.
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "$0")/../.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir "$fixture/bin"
export MERIDIAN_POWER_TEST="$fixture"
cat > "$fixture/bin/gapplication" <<'MOCK'
#!/bin/sh
printf '%s\n' "$*" >> "$MERIDIAN_POWER_TEST/actions"
MOCK
cat > "$fixture/bin/gdbus" <<'MOCK'
#!/bin/sh
case "$(cat "$MERIDIAN_POWER_TEST/mode")" in
  locked) echo "((true, signature '', [<true>]),)" ;;
  unlocked) echo "((true, signature '', [<false>]),)" ;;
  unavailable) exit 1 ;;
esac
MOCK
cat > "$fixture/bin/sleep" <<'MOCK'
#!/bin/sh
exit 0
MOCK
cat > "$fixture/bin/swayidle" <<'MOCK'
#!/bin/sh
printf '%s\n' "$@" > "$MERIDIAN_POWER_TEST/idle-args"
printf '%s\n' "$MERIDIAN_LOCK_HELPER" > "$MERIDIAN_POWER_TEST/helper"
MOCK
cat > "$fixture/bin/meridian-shell" <<'MOCK'
#!/bin/sh
printf '%s\n' "${MERIDIAN_TEST_TIMEOUTS:-300 330}"
MOCK
cat > "$fixture/bin/wlopm" <<'MOCK'
#!/bin/sh
printf '%s\n' "$*" >> "$MERIDIAN_POWER_TEST/output-actions"
MOCK
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"
printf 'locked' > "$fixture/mode"
sh "$ROOT/session/labwc/lock-session"
[ "$(cat "$fixture/actions")" = 'action org.meridian.Shell lock-session' ]
printf 'unlocked' > "$fixture/mode"
if sh "$ROOT/session/labwc/lock-session" 2>/dev/null; then
    echo 'Unconfirmed lock incorrectly accepted' >&2; exit 1
fi
printf 'unavailable' > "$fixture/mode"
if sh "$ROOT/session/labwc/lock-session" 2>/dev/null; then
    echo 'Unavailable shell incorrectly accepted' >&2; exit 1
fi
sh "$ROOT/session/labwc/idle"
mapfile -t args < "$fixture/idle-args"
[ "${args[0]}" = '-w' ]
[ "${args[1]} ${args[2]}" = 'timeout 300' ]
[ "${args[4]} ${args[5]}" = 'timeout 330' ]
[ "${args[7]}" = 'resume' ]
[ "${args[9]}" = 'before-sleep' ]
[ "${args[11]}" = 'after-resume' ]
export MERIDIAN_LOCK_HELPER
MERIDIAN_LOCK_HELPER=$(cat "$fixture/helper")
printf 'unlocked' > "$fixture/mode"
if sh -c "${args[6]}" 2>/dev/null; then
    echo 'Display blanked without a confirmed lock' >&2; exit 1
fi
[ ! -f "$fixture/output-actions" ]
printf 'locked' > "$fixture/mode"
sh -c "${args[6]}"
[ "$(cat "$fixture/output-actions")" = '--off *' ]
sh -c "${args[8]}"
sh -c "${args[12]}"
[ "$(tail -n 1 "$fixture/output-actions")" = '--on *' ]
MERIDIAN_TEST_TIMEOUTS='600 720' sh "$ROOT/session/labwc/idle"
mapfile -t args < "$fixture/idle-args"
[ "${args[2]} ${args[5]}" = '600 720' ]
for value in '0 0' '60 30' '300 999999' '300; touch /tmp/invalid' '300 330 500'; do
    MERIDIAN_TEST_TIMEOUTS="$value" sh "$ROOT/session/labwc/idle"
    mapfile -t args < "$fixture/idle-args"
    [ "${args[2]} ${args[5]}" = '300 330' ]
done
echo 'Power controls: confirmed lock, failure paths, screen standby and wake passed.'
