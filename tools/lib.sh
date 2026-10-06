# Shared helpers for tools/*.sh (sourced, not executed).

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLBOX="${MERIDIAN_TOOLBOX:-meridian-dev}"

# Re-run the calling script inside the dev toolbox when we're on a host that
# lacks the toolchain. Set MERIDIAN_NO_TOOLBOX=1 to build on the host.
ensure_toolchain() {
    if [ -e /run/.toolboxenv ] || [ "${MERIDIAN_NO_TOOLBOX:-0}" = 1 ] || command -v cargo >/dev/null 2>&1; then
        return 0
    fi
    if command -v toolbox >/dev/null 2>&1 && toolbox list --containers 2>/dev/null | grep -q " $TOOLBOX "; then
        exec toolbox run -c "$TOOLBOX" "$@"
    fi
    echo "error: no Rust toolchain found. Run tools/dev-env.sh first (or install the" >&2
    echo "       packages listed in docs/DEVELOPMENT.md on the host)." >&2
    exit 1
}

build_all() {
    (cd "$ROOT/ui" && { [ -d node_modules ] || npm ci --no-fund --no-audit; } && npm run build)
    cargo build --manifest-path "$ROOT/Cargo.toml" -p meridian-shell "$@"
}

# Exec a (nested) compositor command line.
#
# Inside a rootless toolbox the host's /tmp/.X11-unix shows up as owned by
# "nobody", and labwc refuses to start XWayland there (fatal). Give the nested
# session a private /tmp/.X11-unix in its own mount namespace; this affects
# only this process tree, never the host.
exec_compositor() {
    local x11_owner
    x11_owner="$(stat -c %u /tmp/.X11-unix 2>/dev/null || echo 0)"
    if [ -e /run/.toolboxenv ] && [ "$x11_owner" != 0 ] && [ "$x11_owner" != "$(id -u)" ]; then
        exec sudo --preserve-env unshare --mount -- sh -c \
            'uid=$1 gid=$2 home=$3 user=$4; shift 4
             mount -t tmpfs -o mode=1777 meridian-x11 /tmp/.X11-unix &&
             HOME=$home USER=$user LOGNAME=$user \
             exec setpriv --reuid="$uid" --regid="$gid" --init-groups -- "$@"' \
            sh "$(id -u)" "$(id -g)" "$HOME" "$(id -un)" "$@"
    fi
    exec "$@"
}
