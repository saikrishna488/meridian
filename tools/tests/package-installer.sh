#!/usr/bin/env bash
set -euo pipefail
TOOLS="$(cd "$(dirname "$0")/.." && pwd)"
source "$TOOLS/release-manifest.sh"
fixture="$(mktemp -d)"
trap 'rm -rf -- "$fixture"' EXIT
ROOT="$fixture/repo"
mkdir -p "$ROOT"/{protocol,services,shell,ui/{src,scripts,dist},assets,session,tools,target/release}
cp "$TOOLS"/{package-installer.sh,installer-header.sh,lib.sh,release-manifest.sh} "$ROOT/tools/"
for file in Cargo.toml Cargo.lock rustfmt.toml LICENSE ui/package.json ui/package-lock.json ui/tsconfig.json \
    ui/dist/main.js target/release/meridian-shell target/release/meridian-greeter \
    target/release/meridian-drift; do
    printf 'fixture\n' > "$ROOT/$file"
done
cat > "$ROOT/tools/install.sh" <<'INNER'
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/tools/release-manifest.sh"
[ "${1:-}" = --check-build ]
verify_release
echo 'Fixture verified.'
INNER
release_manifest > "$ROOT/target/release/meridian-build.sha256"
bash "$ROOT/tools/package-installer.sh" --no-build --output "$fixture/installer with spaces.run"
bash "$fixture/installer with spaces.run" --check-build
bash "$fixture/installer with spaces.run" --help > /dev/null
if bash "$fixture/installer with spaces.run" --invalid 2>/dev/null; then exit 1; fi
cp "$fixture/installer with spaces.run" "$fixture/damaged.run"
printf x >> "$fixture/damaged.run"
if bash "$fixture/damaged.run" --check-build 2>/dev/null; then echo 'Accepted corrupt payload' >&2; exit 1; fi
printf 'changed\n' >> "$ROOT/ui/dist/main.js"
if bash "$ROOT/tools/package-installer.sh" --no-build --output "$fixture/stale.run" 2>/dev/null; then
    echo 'Packaged stale release' >&2; exit 1
fi
echo 'Single-file installer tests passed.'
