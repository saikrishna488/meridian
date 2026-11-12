#!/usr/bin/env bash
# Run the Swift/C++ workspace without starting the Rust or React shell.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
configuration="${MERIDIAN_BUILD_CONFIGURATION:-debug}"
binary="$root/target/swift-ui/$configuration/meridian-native-applications"
if [[ ! -x $binary ]]; then
    echo "Build first: MERIDIAN_BUILD_CONFIGURATION=$configuration bash tools/build-native.sh" >&2
    exit 1
fi
# Support the optional project-local Fedora toolchain used in development.
local_runtime="$root/target/toolchains/swift/usr/lib64"
if [[ -f $local_runtime/libswiftCore.so ]]; then
    export LD_LIBRARY_PATH="$local_runtime${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
exec "$binary" --desktop "$@"
