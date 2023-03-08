# Variables

### Windows
handle_wsl () {
	local system_info="$(cat /proc/version | tr '[:upper:]' '[:lower:]')";
	if [[ "$system_info" == *"wsl"* ]]; then
		win_main_drive_path="/mnt/c";

		p_win_home="$win_main_drive_path/p-win";
	fi
}

handle_wsl

### Linux
#### Paths
p_home="$HOME/p";
notes_home="$p_home/notes";

scripts_home=$HOME/scripts;

### Standard Programs
export standard_editor=nvim;
