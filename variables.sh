# Variables

### Windows
if [[ $IS_WSL == true ]]; then
	win_main_drive_path="/mnt/c";

	p_win_home="$win_main_drive_path/p-win";
fi

### Linux
#### Paths
p_home="$HOME/p";
notes_home="$p_home/notes";

scripts_home=~/scripts;

### Standard Programs
export standard_editor=nvim;