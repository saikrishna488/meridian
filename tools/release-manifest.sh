Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
Failed to create stream fd: Operation not permitted
# Release integrity helpers, sourced after tools/lib.sh or ROOT is defined.
# Hash both inputs and outputs: an existing binary can still be a stale build.
release_sources() {
    (
        cd "$ROOT"
        find protocol services shell ui/src ui/scripts assets session tools -type f -print0
        printf '%s\0' Cargo.toml Cargo.lock rustfmt.toml ui/package.json ui/package-lock.json ui/tsconfig.json
    )
}

release_source_manifest() {
    (cd "$ROOT"; release_sources | LC_ALL=C sort -z | xargs -0 sha256sum)
}

release_manifest() {
    (
        cd "$ROOT"
        {
            release_sources
            printf '%s\0' target/release/meridian-shell target/release/meridian-greeter target/release/wayland-pipewire-idle-inhibit
            find ui/dist -type f -print0
        } | LC_ALL=C sort -z | xargs -0 sha256sum
    )
}

verify_release() {
    local manifest="$ROOT/target/release/meridian-build.sha256" current
    if [ ! -f "$manifest" ]; then
        echo "error: no verified release build; run tools/compile_meridian.sh as your user first" >&2
        return 1
    fi
    current="$(release_manifest)" || return 1
    if [ "$current" != "$(cat "$manifest")" ]; then
        echo "error: release binaries, UI, or source files changed since the release build" >&2
        echo "Rebuild with tools/compile_meridian.sh before packaging or installing Meridian." >&2
        return 1
    fi
}
