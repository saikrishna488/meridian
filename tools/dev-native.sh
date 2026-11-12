#!/usr/bin/env bash
# Run the Swift/C++ workspace without starting the Rust or React shell.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
configuration="${MERIDIAN_BUILD_CONFIGURATION:-debug}"
if [[ -z ${MERIDIAN_BUILD_CONFIGURATION:-} && -z ${MERIDIAN_NATIVE_BINARY:-} ]]; then
    debug="$root/target/swift-ui/debug/meridian-native-applications"
    release="$root/target/swift-ui/release/meridian-native-applications"
    if [[ -x $release && ( ! -x $debug || $release -nt $debug ) ]]; then configuration=release; fi
fi
binary="${MERIDIAN_NATIVE_BINARY:-$root/target/swift-ui/$configuration/meridian-native-applications}"
if [[ ! -x $binary ]]; then
    echo "Native executable not found: $binary" >&2
    echo "Build first: MERIDIAN_BUILD_CONFIGURATION=$configuration bash tools/build-native.sh" >&2
    exit 1
fi
# Support the optional project-local Fedora toolchain used in development.
local_runtime="$root/target/toolchains/swift/usr/lib64"
if [[ -f $local_runtime/libswiftCore.so ]]; then
    export LD_LIBRARY_PATH="$local_runtime${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
if [[ ${1:-} == --check ]]; then
    if ! dependencies=$(LC_ALL=C ldd "$binary" 2>&1) || [[ $dependencies == *'not found'* ]]; then
        printf 'Native runtime dependencies are unavailable:\n%s\n' "$dependencies" >&2
        exit 1
    fi
    exit 0
fi
if [[ ${MERIDIAN_NATIVE_PRIVATE_BUS:-0} == 1 ]] && command -v dbus-update-activation-environment >/dev/null; then
    # Update only the private bus, never the host's systemd activation environment.
    dbus-update-activation-environment WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE 2>/dev/null || true
fi
exec "$binary" --desktop "$@"
