#!/usr/bin/env bash
# Refresh the redistributable first-launch GUI without building the web UI.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "${1:-}" in
  '') CARGO_BUILD_JOBS=1 cargo build --locked --release --manifest-path "$ROOT/Cargo.toml" -p meridian-installer ;;
  --from-release) ;;
  *) echo "Usage: $0 [--from-release]" >&2; exit 1 ;;
esac
[[ $(uname -s) == Linux ]] || { echo 'Package the installer on Linux.' >&2; exit 1; }
architecture="$(uname -m)"
case "$architecture" in x86_64|aarch64) ;; *) echo "Unsupported installer architecture: $architecture" >&2; exit 1 ;; esac
bundle="meridian-installer-linux-$architecture"
mkdir -p "$ROOT/tools/bin"
install -m755 "$ROOT/target/release/meridian-installer" "$ROOT/tools/bin/$bundle"
(cd "$ROOT/tools/bin"; sha256sum "$bundle" > "$bundle.sha256")
printf 'Bundled GUI: tools/bin/%s\n' "$bundle"
