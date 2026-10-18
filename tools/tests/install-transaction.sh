#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
export TMPDIR="$fixture"
printf 'old binary\n' > "$fixture/shell"
mkdir "$fixture/ui"
printf 'old ui\n' > "$fixture/ui/index.html"
export fixture ROOT
if bash -eu -c '
    source "$ROOT/tools/install-transaction.sh"
    meridian_transaction_begin "$fixture/shell" "$fixture/ui" "$fixture/new-session"
    printf "new binary\n" > "$fixture/shell"
    printf "new ui\n" > "$fixture/ui/index.html"
    printf "new session\n" > "$fixture/new-session"
    exit 13
' 2> "$fixture/rollback.log"; then
    echo 'Expected simulated installation failure' >&2; exit 1
fi
[ "$(cat "$fixture/shell")" = 'old binary' ]
[ "$(cat "$fixture/ui/index.html")" = 'old ui' ]
[ ! -e "$fixture/new-session" ]
bash -eu -c '
    source "$ROOT/tools/install-transaction.sh"
    meridian_transaction_begin "$fixture/new-session"
    printf "clean install\n" > "$fixture/new-session"
    meridian_transaction_commit
'
[ "$(cat "$fixture/new-session")" = 'clean install' ]
echo 'Clean file installation and rollback after partial replacement passed.'
