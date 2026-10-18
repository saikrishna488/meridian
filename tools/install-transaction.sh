# File rollback for install.sh. Package-manager changes are outside this scope.
meridian_transaction_begin() {
    MERIDIAN_TRANSACTION_DIR=$(mktemp -d "${TMPDIR:-/var/tmp}/meridian-install.XXXXXX")
    chmod 700 "$MERIDIAN_TRANSACTION_DIR"
    MERIDIAN_TRANSACTION_PATHS=("$@")
    MERIDIAN_TRANSACTION_PRESENT=()
    local index=0 path
    for path in "${MERIDIAN_TRANSACTION_PATHS[@]}"; do
        if [ -e "$path" ] || [ -L "$path" ]; then
            cp -a -- "$path" "$MERIDIAN_TRANSACTION_DIR/original-$index"
            MERIDIAN_TRANSACTION_PRESENT+=(yes)
        else
            MERIDIAN_TRANSACTION_PRESENT+=(no)
        fi
        index=$((index + 1))
    done
    # Arm only after every existing path was successfully copied.
    trap 'meridian_transaction_exit $?' EXIT
}
meridian_transaction_exit() {
    local status="$1" index=0 path failed=0
    trap - EXIT
    [ "$status" -ne 0 ] || return 0
    set +e
    for path in "${MERIDIAN_TRANSACTION_PATHS[@]}"; do
        if [ -e "$path" ] || [ -L "$path" ]; then
            mv -- "$path" "$MERIDIAN_TRANSACTION_DIR/failed-$index" || failed=1
        fi
        if [ "${MERIDIAN_TRANSACTION_PRESENT[$index]}" = yes ]; then
            cp -a -- "$MERIDIAN_TRANSACTION_DIR/original-$index" "$path" || failed=1
        fi
        index=$((index + 1))
    done
    if [ "$failed" -eq 0 ]; then
        echo "Installation failed; the previous Meridian files were restored. Backup: $MERIDIAN_TRANSACTION_DIR" >&2
    else
        echo "Installation rollback needs manual repair. Preserve $MERIDIAN_TRANSACTION_DIR and inspect its original-* files." >&2
    fi
    # Preserve all snapshots and failed files for diagnosis after an error.
    exit "$status"
}
meridian_transaction_commit() {
    trap - EXIT
    # This path is created by mktemp above and contains only installation snapshots.
    rm -rf -- "$MERIDIAN_TRANSACTION_DIR"
    unset MERIDIAN_TRANSACTION_DIR MERIDIAN_TRANSACTION_PATHS MERIDIAN_TRANSACTION_PRESENT
}
