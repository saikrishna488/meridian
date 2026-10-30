#!/usr/bin/env bash
# Everything CI should run: formatting, lints, tests, UI type-check, and a
# check that the generated TypeScript protocol types are up to date.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"
cd "$ROOT"

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "release integrity"
bash tools/tests/distro.sh
bash tools/tests/release-manifest.sh
bash tools/tests/screenshot.sh
bash tools/tests/power.sh
bash tools/tests/session.sh
bash tools/tests/install-transaction.sh
bash tools/tests/source-install.sh
python3 tools/tests/profile-desktop.py
python3 tools/tests/check-bindings.py

step "rustfmt"
cargo fmt --all --check

step "clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "generated protocol bindings match clean deterministic exports"
python3 tools/check-bindings.py

step "tests (also regenerates ui/src/lib/generated)"
cargo test --workspace

step "UI type-check, build, and interaction tests"
(cd ui && { [ -d node_modules ] || npm ci --no-fund --no-audit; } && npm test)

echo; echo "All checks passed."
