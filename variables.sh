# Variables

### Windows (uses IS_WSL set by determine_running_os in general_functions.sh)
if [[ "$IS_WSL" == "true" ]]; then
	win_main_drive_path="/mnt/c";
	p_win_home="$win_main_drive_path/p";
fi

### Linux
#### Paths
p_home="$HOME/p";
notes_home="$p_home/notes";

scripts_home=$HOME/scripts;

### Standard Programs
export standard_editor=nvim;
