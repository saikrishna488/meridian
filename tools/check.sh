#!/usr/bin/env bash
# Everything CI should run: formatting, lints, tests, UI type-check, and a
# check that the generated TypeScript protocol types are up to date.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
ensure_toolchain "$0" "$@"
cd "$ROOT"

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

step "rustfmt"
cargo fmt --all --check

step "clippy"
cargo clippy --workspace --all-targets -- -D warnings

step "tests (also regenerates ui/src/lib/generated)"
cargo test --workspace

step "generated protocol types are committed"
if git rev-parse --verify -q HEAD >/dev/null && [ -n "$(git status --porcelain -- ui/src/lib/generated)" ]; then
    git status --short -- ui/src/lib/generated
    echo "error: generated TypeScript differs from the committed copy; commit it" >&2
    exit 1
fi

step "UI type-check and build"
(cd ui && { [ -d node_modules ] || npm ci --no-fund --no-audit; } && npm run check && npm run build)

echo; echo "All checks passed."
