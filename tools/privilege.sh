# Source from user-facing setup/build scripts. The GUI never handles passwords.
meridian_privilege_runner() {
    MERIDIAN_PRIVILEGE=(sudo)
    if [ "${MERIDIAN_GUI_AUTH:-0}" = 1 ]; then
        command -v pkexec >/dev/null 2>&1 || {
            echo 'Graphical authentication requires pkexec and a running desktop Polkit agent. Install them or use the terminal installer.' >&2
            return 1
        }
        # The GUI has no controlling terminal. Never launch pkexec's text agent.
        MERIDIAN_PRIVILEGE=(meridian_graphical_privilege)
    else
        command -v sudo >/dev/null 2>&1 || { echo 'sudo is required for system installation.' >&2; return 1; }
    fi
}

meridian_graphical_privilege() {
    local status
    if pkexec --disable-internal-agent "$@"; then
        return 0
    else
        status=$?
    fi
    if [ "$status" = 126 ] || [ "$status" = 127 ]; then
        echo 'Administrator authentication was cancelled, failed, or the graphical Polkit agent is unavailable. If its window disappeared, the agent may have crashed. Install mate-polkit from a terminal, then sign out and back in. You can also run bash install.sh --cli in a terminal. No privileged command was retried automatically.' >&2
    fi
    return "$status"
}
