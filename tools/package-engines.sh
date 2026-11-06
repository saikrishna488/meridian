#!/usr/bin/env bash
# Compile four compact C++ static libraries; never run tests or build the web UI.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
architecture="$(uname -m)"
case "$architecture" in x86_64|aarch64) ;; *) echo "Unsupported native engine architecture: $architecture" >&2; exit 1 ;; esac
output="$ROOT/shell/native/prebuilt/$architecture"
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
mkdir -p "$output"
read -r -a includes <<< "$(pkg-config --cflags fontconfig freetype2 pangocairo)"
for engine in font gpu cpu audio; do
  case "$engine" in gpu) source=graphics ;; cpu) source=performance ;; *) source="$engine" ;; esac
  "${CXX:-c++}" -std=c++17 -Os -fPIC -fno-ident -ffunction-sections -fdata-sections -fvisibility=hidden \
    "${includes[@]}" -c "$ROOT/shell/native/engines/$source.cpp" -o "$staging/$engine.o"
  "${STRIP:-strip}" --strip-debug "$staging/$engine.o"
  "${AR:-ar}" crsD "$staging/libmeridian-$engine.a" "$staging/$engine.o"
  install -m644 "$staging/libmeridian-$engine.a" "$output/libmeridian-$engine.a"
done
(cd "$ROOT/shell"; sha256sum native/engines/*.cpp native/engines/engines.h native/prebuilt/"$architecture"/*.a > "$staging/manifest.sha256")
install -m644 "$staging/manifest.sha256" "$output/manifest.sha256"
printf 'Precompiled engine libraries (%s):\n' "$architecture"
wc -c "$output"/*.a
