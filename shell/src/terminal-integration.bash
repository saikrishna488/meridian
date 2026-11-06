# Optional Meridian Bash integration. Preserve the user's startup and prompt.
[[ -f ~/.bashrc ]] && source ~/.bashrc
__meridian_prompt() {
    local result=$?
    printf '\033]133;D;%s\007' "$result"
    printf '\033]133;A\007'
    return "$result"
}
PS0=$'\e]133;C\a'"${PS0-}"
if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == 'declare -a'* ]]; then
    PROMPT_COMMAND=(__meridian_prompt "${PROMPT_COMMAND[@]}")
else
    PROMPT_COMMAND=(__meridian_prompt "${PROMPT_COMMAND-}")
fi
