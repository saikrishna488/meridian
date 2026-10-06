#!/usr/bin/env bash
# Build optimized binaries and the UI bundle for tools/install.sh.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"
build_all --release --bin meridian-shell --bin meridian-greeter
echo "Built. Install with: sudo tools/install.sh"
