# shellcheck shell=bash
# My bash customization
## Bash customization home
export bashC="${BASHC_ROOT:-$HOME/bashCustomization}";
export BASHC_ROOT="$bashC"
# Used by dynamically sourced local modules.
# shellcheck disable=SC2034
local_dir="$bashC/local";

# BASHC_ROOT deliberately supports alternate roots.
# shellcheck disable=SC1091
if ! . "$bashC/general_functions.sh"; then
    printf 'bashc: failed to load core shell functions\n' >&2
    return 1
fi

determine_running_os || return 1

# Checks for shell version and saves it in system variable.
determine_running_shell || return 1

## Loading extending files
if ! load_shell_extentionfiles "first_load"; then
    printf 'bashc: shell customization failed to load\n' >&2
    return 1
fi

# Fetch updates once per day, then reload if something changed
check_for_shell_update_once_a_day () {
    if [[ -n ${BASHC_SKIP_UPDATE_CHECK:-} ]]; then
        return 0
    fi

    local current_date
    current_date=$(date +%F) || return 1
    local path_to_shell_update="${BASHC_UPDATE_STATE_FILE:-$bashC/.last_day_shell_update_checked}"
    local last_date_shell_checked=""

    if [[ -f "$path_to_shell_update" ]]; then
        last_date_shell_checked=$(<"$path_to_shell_update")
    fi

    if [[ "$current_date" == "$last_date_shell_checked" ]]; then
        return 0
    fi

    if ! git_pull_repo "$bashC"; then
        printf 'bashc: daily shell update failed; will retry next shell start\n' >&2
        return 1
    fi

    if ! load_shell_extentionfiles; then
        printf 'bashc: updated files could not be reloaded; will retry next shell start\n' >&2
        return 1
    fi

    printf '%s\n' "$current_date" > "$path_to_shell_update"
}

# A network or reload failure must not prevent the already-installed shell
# configuration (including the local config drift check) from loading. The
# unchanged date marker makes the updater retry on the next shell start.
if ! check_for_shell_update_once_a_day; then
    true
fi

bashc_check_configs
