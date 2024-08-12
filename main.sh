# My bash customization
## Bash customization home
export bashC="$HOME/bashCustomization";
local_dir="$bashC/local";

. $bashC/general_functions.sh;

determine_running_os;

# Checks for shell version and saves it in system variable.
determine_running_shell;

#if ! [ -f ~/.vim/.vimrc ]; then
#	echo "Running vim setup";
#	. $bashC/vim/setupVim.sh;
#fi

## Loading extending files
load_shell_extentionfiles "false";

# Fetch any new thing whenever the shell is started on tuesday or thursday
check_for_shell_update_on_monday () {
    local current_date_nunber=$(date +%u);
    local path_to_shell_update="$bashC/.last_day_shell_update_checked";

    if [[ ! -f $bashC/.last_day_shell_update_checked ]]; then
        echo $current_date_nunber > $path_to_shell_update;
        updateShell;
        return 0;
    fi

    local last_day_shell_checked=$(cat $path_to_shell_update);

    if [[ $current_date_nunber != $last_day_shell_checked ]]; then
        echo $current_date_nunber > $path_to_shell_update;
        updateShell;
        return 0;
    fi
}

check_for_shell_update_on_monday

load_shell_extentionfiles "first_load";
