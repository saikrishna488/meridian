#!/usr/bin/env bash
# Exercise the swaylock hand-off and idle commands without locking the
# developer's session: a swaylock mock stands in for the real lock screen.
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "$0")/../.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir "$fixture/bin" "$fixture/no-swaylock"
export MERIDIAN_POWER_TEST="$fixture"
cat > "$fixture/bin/swaylock" <<'MOCK'
#!/bin/sh
printf '%s\n' "$*" >> "$MERIDIAN_POWER_TEST/swaylock-args"
exit "${MERIDIAN_TEST_SWAYLOCK_EXIT:-0}"
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

# Locking is delegated to swaylock with stable flags.
sh "$ROOT/session/labwc/lock-session"
[ "$(cat "$fixture/swaylock-args")" = '--daemonize --color 000000' ]

# Without swaylock on PATH the lock is refused rather than reported as held.
if PATH="$fixture/no-swaylock" /bin/sh "$ROOT/session/labwc/lock-session" >/dev/null 2>&1; then
    echo 'Lock without swaylock incorrectly accepted' >&2; exit 1
fi
# A lock that fails to start (for example, rejected authentication) is refused.
if MERIDIAN_TEST_SWAYLOCK_EXIT=1 sh "$ROOT/session/labwc/lock-session" >/dev/null 2>&1; then
    echo 'Failed lock incorrectly accepted' >&2; exit 1
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
# Display standby waits for a confirmed lock.
if MERIDIAN_TEST_SWAYLOCK_EXIT=1 sh -c "${args[6]}" 2>/dev/null; then
    echo 'Display blanked without a confirmed lock' >&2; exit 1
fi
[ ! -f "$fixture/output-actions" ]
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
echo 'Power controls: lock hand-off, refusal paths, screen standby and wake passed.'
