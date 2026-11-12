#!/usr/bin/env bash
# Test only in private headless labwc; no input reaches the user's desktop.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
build="$root/target/native-pointer"
mkdir -p "$build"
protocol="$root/tools/tests/protocols/wlr-virtual-pointer-unstable-v1.xml"
wayland-scanner client-header "$protocol" "$build/virtual-pointer-client.h"
wayland-scanner private-code "$protocol" "$build/virtual-pointer-protocol.c"
read -r -a flags <<< "$(pkg-config --cflags --libs wayland-client)"
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -I "$build" \
    "$root/tools/tests/native-pointer.c" "$build/virtual-pointer-protocol.c" \
    "${flags[@]}" -o "$build/native-pointer"
local_runtime="$root/target/toolchains/swift/usr/lib64"
if [[ -f $local_runtime/libswiftCore.so ]]; then
    export LD_LIBRARY_PATH="$local_runtime${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
wtype="${MERIDIAN_WTYPE:-wtype}"
if ! command -v "$wtype" >/dev/null && [[ -x $root/target/toolchains/wtype/usr/bin/wtype ]]; then
    wtype="$root/target/toolchains/wtype/usr/bin/wtype"
fi
exec python3 "$root/tools/tests/native-workspace.py" \
    --pointer "$build/native-pointer" --wtype "$wtype" "$@"
