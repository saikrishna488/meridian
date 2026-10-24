#!/usr/bin/env bash
# Build a single distributable file, preserving the existing release verifier.
set -euo pipefail
source "$(dirname "$0")/lib.sh"
source "$ROOT/tools/release-manifest.sh"
build=true
output="$ROOT/target/meridian-installer.run"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-build) build=false; shift ;;
        --output) [ "$#" -ge 2 ] || { echo 'Missing output path.' >&2; exit 2; }; output="$2"; shift 2 ;;
        --help|-h) echo 'Usage: tools/package-installer.sh [--no-build] [--output FILE]'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done
output="$(realpath -m "$output")"
# Output must not become a source input or overwrite a build artifact.
case "$output" in
    "$ROOT/target/meridian-installer.run") ;;
    "$ROOT"/*) echo 'Use the default output or a path outside the checkout.' >&2; exit 2 ;;
esac
if "$build"; then bash "$ROOT/tools/build-release.sh"; fi
verify_release
bundle_stage="$(mktemp -d)"
trap 'rm -rf -- "$bundle_stage"' EXIT
{
    release_sources
    printf '%s\0' LICENSE target/release/meridian-build.sha256 \
        target/release/meridian-shell target/release/meridian-greeter \
        target/release/meridian-drift ui/dist
} > "$bundle_stage/files"
tar -czf "$bundle_stage/payload.tar.gz" -C "$ROOT" --no-recursion \
    protocol services shell ui/src ui/scripts assets session tools \
    --recursion --null -T "$bundle_stage/files"
# Verify the snapshot too, in case a source changed while packaging.
mkdir "$bundle_stage/check"
tar -xzf "$bundle_stage/payload.tar.gz" -C "$bundle_stage/check"
(ROOT="$bundle_stage/check"; verify_release)
payload_hash="$(sha256sum "$bundle_stage/payload.tar.gz" | cut -d ' ' -f1)"
sed "s/@PAYLOAD_SHA256@/$payload_hash/" "$ROOT/tools/installer-header.sh" > "$bundle_stage/installer.run"
cat "$bundle_stage/payload.tar.gz" >> "$bundle_stage/installer.run"
mkdir -p "$(dirname "$output")"
install -m755 "$bundle_stage/installer.run" "$output"
echo "Created $output"
echo "Install: sudo bash '$output'"
