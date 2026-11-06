# Release integrity helpers, sourced after tools/lib.sh or ROOT is defined.
# Hash both inputs and outputs: an existing binary can still be a stale build.
release_sources() {
    (
        cd "$ROOT"
        find protocol services shell installer ui/src ui/scripts assets session tools -type f ! -path 'tools/bin/*' -print0
        printf '%s\0' install.sh Cargo.toml Cargo.lock rustfmt.toml ui/package.json ui/package-lock.json ui/tsconfig.json
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
            if [ -d tools/bin ]; then find tools/bin -type f -print0; fi
            printf '%s\0' target/release/meridian-shell target/release/meridian-greeter target/release/meridian-installer target/release/wayland-pipewire-idle-inhibit
            find ui/dist -type f -print0
        } | LC_ALL=C sort -z | xargs -0 sha256sum
    )
}

verify_release() {
    local manifest="$ROOT/target/release/meridian-build.sha256" current artifact
    for artifact in meridian-shell meridian-greeter meridian-installer wayland-pipewire-idle-inhibit; do
        if [ ! -f "$ROOT/target/release/$artifact" ] || [ ! -r "$ROOT/target/release/$artifact" ]; then
            echo "error: the release is incomplete: target/release/$artifact is missing or unreadable" >&2
            echo "Run as your regular user: bash install.sh --cli --update" >&2
            echo "This compiles all required binaries and installs the verified release." >&2
            return 1
        fi
    done
    if [ ! -f "$manifest" ]; then
        echo "error: no verified release build; run bash install.sh --cli --compile as your user first" >&2
        return 1
    fi
    current="$(release_manifest)" || return 1
    if [ "$current" != "$(cat "$manifest")" ]; then
        echo "error: release binaries, UI, or source files changed since the release build" >&2
        echo "Rebuild with bash install.sh --cli --compile before packaging or installing Meridian." >&2
        return 1
    fi
}
