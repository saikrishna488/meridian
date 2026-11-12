#!/usr/bin/env bash
# Native backend checks require no Swift compiler, WebKit or Rust runtime.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
surfaces=ON
sanitizers=OFF
wayland=0
build="$root/target/native"
for option in "$@"; do
    case "$option" in
        --backend-only) surfaces=OFF; build="$root/target/native-backend" ;;
        --sanitize) sanitizers=ON ;;
        --wayland) wayland=1 ;;
        *) echo 'Usage: check-native.sh [--backend-only] [--sanitize] [--wayland]' >&2; exit 2 ;;
    esac
done
if [[ $surfaces == OFF && $wayland == 1 ]]; then
    echo '--wayland requires surface integration.' >&2
    exit 2
fi
if [[ $sanitizers == ON ]]; then build="$build-sanitize"; fi
python3 "$root/tools/migration-inventory.py" --check
python3 "$root/tools/tests/native-inventory.py"
cmake -S "$root/native" -B "$build" -DCMAKE_BUILD_TYPE=RelWithDebInfo \
    -DMERIDIAN_BUILD_SURFACES="$surfaces" -DMERIDIAN_SANITIZERS="$sanitizers"
cmake --build "$build" --parallel "${MERIDIAN_BUILD_JOBS:-2}"
ctest --test-dir "$build" --output-on-failure
if [[ $wayland == 1 ]]; then
    python3 "$root/tools/tests/native-wayland.py"
fi
