# This function takes a string as the first parameter, and a second string as the second.
# It then checks the first string for any occurance of the first string, and then returns the exit code of grep.
check_param_for_string () {
	echo $1 | grep $2 &> /dev/null;
	return;
}

determine_running_os () {
	# Check if it is running on mac
	if [[ "$OSTYPE" == *"darwin"* ]]; then
		IS_MAC=true;
	else
		IS_MAC=false;
	fi

	# Check if it is running in wsl
	cat /proc/version | tr '[:upper:]' '[:lower:]' | grep wsl &> /dev/null;
	if [[ $? -eq 0 ]]; then
		IS_WSL=true;
	else
		IS_WSL=false;
	fi
}

# Checks for shell version and saves it in system variable.
determine_running_shell () {
	if test -n "$ZSH_VERSION"; then
		PROFILE_SHELL=zsh
	elif test -n "$BASH_VERSION"; then
		PROFILE_SHELL=bash
	elif test -n "$KSH_VERSION"; then
		PROFILE_SHELL=ksh
	elif test -n "$FCEDIT"; then
		PROFILE_SHELL=ksh
	elif test -n "$PS3"; then
		PROFILE_SHELL="unknown"
	else
		PROFILE_SHELL=sh
	fi
}

load_shell_extentionfiles () {
	source $bashC/variables.sh &&
	source $bashC/shellFunctionality/shellMain.sh &&
	source $bashC/standard_settings.sh &&
	source $bashC/installScripts/installMain.sh &&
	source $bashC/programExtensions/extentionsMain.sh &&
	source "$local_dir/local_main.sh" &&
	if [[ $# -eq 0 ]]; then
		echo "Done reloading files!";
	else
		echo "Extentions loaded!"
	fi
}
