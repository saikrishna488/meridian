#!/usr/bin/env bash
# Opt-in migration build; does not replace the installed desktop session.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cmake -S "$root/native" -B "$root/target/native" -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build "$root/target/native" --parallel
ctest --test-dir "$root/target/native" --output-on-failure
if ! command -v swift >/dev/null; then
    echo 'C++ backend verified. Swift 6+ is required to build the SwiftCrossUI launcher.' >&2
    exit 1
fi
export SCUI_DEFAULT_BACKEND=GtkBackend
swift package --package-path "$root/swift-ui" --scratch-path "$root/target/swift-ui" resolve
checkout="$root/target/swift-ui/checkouts/swift-cross-ui"
for patch_file in "$root/swift-ui/Patches/"*.patch; do
    if git -C "$checkout" apply --check "$patch_file" 2>/dev/null; then
        git -C "$checkout" apply "$patch_file"
    elif ! git -C "$checkout" apply --reverse --check "$patch_file" 2>/dev/null; then
        echo "SwiftCrossUI patch no longer matches the pinned dependency: $patch_file" >&2
        exit 1
    fi
done
read -r -a gio_flags <<< "$(pkg-config --libs-only-L --libs-only-l gio-unix-2.0 libsystemd gtk4 gtk4-layer-shell-0)"
swift_flags=(-Xlinker "-L$root/target/native" -Xlinker -lmeridian-native -Xlinker -lmeridian-surfaces -Xlinker -lstdc++)
# SwiftPM does not track external CMake archives as build inputs. Change a
# link-only flag when their contents change so incremental builds cannot run
# stale native code. GNU ld supports this absolute metadata symbol on Linux.
native_hash=$(sha256sum "$root/target/native/libmeridian-native.a" "$root/target/native/libmeridian-surfaces.a" | sha256sum)
swift_flags+=(-Xlinker "--defsym=meridian_native_build=0x${native_hash:0:15}")
for flag in "${gio_flags[@]}"; do swift_flags+=(-Xlinker "$flag"); done
# Swift 6.3's TBD validator misclassifies the GLib 2.88 GDBus constants used
# by the pinned Gtk backend. Apply only to that compiler/library combination.
if [[ $(swift --version) == *"Swift version 6.3."* ]] && pkg-config --atleast-version=2.88 glib-2.0; then
    echo "Applying Swift 6.3 / GLib 2.88 TBD validation workaround."
    swift_flags+=(-Xswiftc -Xfrontend -Xswiftc -validate-tbd-against-ir=none)
fi
swift_command=build
if [[ ${MERIDIAN_NATIVE_TESTS:-0} == 1 ]]; then swift_command=test; fi
swift "$swift_command" --package-path "$root/swift-ui" --scratch-path "$root/target/swift-ui" \
    -c "${MERIDIAN_BUILD_CONFIGURATION:-release}" --jobs "${MERIDIAN_BUILD_JOBS:-2}" "${swift_flags[@]}" "$@"
