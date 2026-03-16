# My bash customization
## Bash customization home
export bashC="$HOME/bashCustomization";
local_dir="$bashC/local";

. "$bashC/general_functions.sh";

determine_running_os;

# Checks for shell version and saves it in system variable.
determine_running_shell;

## Loading extending files
load_shell_extentionfiles "first_load";

# Fetch updates once per day, then reload if something changed
check_for_shell_update_once_a_day () {
    local current_date_number
    current_date_number=$(date +%u)
    local path_to_shell_update="$bashC/.last_day_shell_update_checked"

    if [[ ! -f "$path_to_shell_update" ]]; then
        echo "$current_date_number" > "$path_to_shell_update"
        updateShell
        load_shell_extentionfiles
        return 0
    fi

    local last_day_shell_checked
    last_day_shell_checked=$(cat "$path_to_shell_update")

    if [[ "$current_date_number" != "$last_day_shell_checked" ]]; then
        echo "$current_date_number" > "$path_to_shell_update"
        updateShell
        load_shell_extentionfiles
        return 0
    fi
}

check_for_shell_update_once_a_day
