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

# Fetch any new thing whenever the shell is started
updateShell;

load_shell_extentionfiles "first_load";
