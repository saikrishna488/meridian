#!/usr/bin/env bash
# Build optimized binaries and the UI bundle for tools/install.sh.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"
source "$ROOT/tools/release-manifest.sh"
rm -f "$ROOT/target/release/meridian-build.sha256"
sources_before="$(release_source_manifest)"
build_all --release --bin meridian-shell --bin meridian-greeter --bin meridian-drift
if [ "$sources_before" != "$(release_source_manifest)" ]; then
    echo "error: source files changed during the build; run tools/build-release.sh again" >&2
    exit 1
fi
release_manifest > "$ROOT/target/release/meridian-build.sha256"
echo "Built. Install with: sudo tools/install.sh"
