#!/usr/bin/env bash
# Regression: installation must reject an old backend paired with a new UI.
set -euo pipefail
source "$(dirname "$0")/../release-manifest.sh"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
mkdir -p "$ROOT"/{protocol,services,shell,ui/{src,scripts,dist},assets,session,tools,target/release}
for file in Cargo.toml Cargo.lock rustfmt.toml ui/package.json ui/package-lock.json ui/tsconfig.json \
    shell/main.rs ui/src/main.ts ui/dist/main.js target/release/meridian-shell target/release/meridian-greeter; do
    printf 'original\n' > "$ROOT/$file"
done
reject() {
    if verify_release 2>/dev/null; then
        echo "error: accepted an unverified or mixed release" >&2
        exit 1
    fi
}
reject
release_manifest > "$ROOT/target/release/meridian-build.sha256"
verify_release
for file in shell/main.rs ui/src/main.ts ui/dist/main.js target/release/meridian-shell; do
    printf 'changed\n' > "$ROOT/$file"
    reject
    printf 'original\n' > "$ROOT/$file"
    verify_release
done
printf 'new file\n' > "$ROOT/ui/dist/new.js"
reject
rm "$ROOT/ui/dist/new.js"
verify_release
rm "$ROOT/target/release/meridian-shell"
reject
echo "Release integrity tests passed."
