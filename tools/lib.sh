# Shared helpers for tools/*.sh (sourced, not executed).

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Build on the Debian host; tools/dev-env.sh installs the native dependencies.
ensure_toolchain() {
    command -v cargo >/dev/null 2>&1 || {
        echo "error: no Rust toolchain found. Run tools/dev-env.sh first." >&2
        exit 1
    }
}

build_all() {
    (cd "$ROOT/ui" && { [ -d node_modules ] || npm ci --no-fund --no-audit; } && npm run build)
    cargo build --manifest-path "$ROOT/Cargo.toml" -p meridian-shell "$@"
}

# Run the nested Wayland compositor on the Debian host.
exec_compositor() { exec "$@"; }
