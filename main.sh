# My bash customization
## Bash customization home
export bashC="$HOME/bashCustomization";
local_dir="$HOME/bashCustomization/local";

. $bashC/general_functions.sh;

determine_running_os;

# Checks for shell version and saves it in system variable.
determine_running_shell;

if ! [ -f ~/.vim/.vimrc ]; then
	echo "Running vim setup";
	. $bashC/vim/setupVim.sh;
fi

## Loading extending files
load_shell_extentionfiles true

# Fetch any new thing whenever the shell is started
updateShell;
