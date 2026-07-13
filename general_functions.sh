# This function takes a string as the first parameter, and a second string as the second.
# It then checks the first string for any occurance of the first string, and then returns the exit code of grep.
check_param_for_string () {
	if [[ $1 == *"$2"* ]]; then
		return 0;
	fi
	return 1;
}

determine_running_os () {
	# Check if it is running on mac
	if [[ "$OSTYPE" == *"darwin"* ]]; then
		IS_MAC=true;
		IS_WSL=false;
		return 0;
	else
		IS_MAC=false;
	fi

	# Check if it is running in wsl
	local system_info="$(cat /proc/version | tr '[:upper:]' '[:lower:]')";
	if [[ "$system_info" == *"wsl"* ]]; then
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

_bashc_source_file () {
	local source_path
	source_path=$1

	if [ ! -r "$source_path" ]; then
		printf 'bashc: required shell module is not readable: %s\n' "$source_path" >&2
		return 1
	fi

	if ! . "$source_path"; then
		printf 'bashc: failed to source required shell module: %s\n' "$source_path" >&2
		return 1
	fi
}

load_shell_extentionfiles () {
	_bashc_source_file "$bashC/variables.sh" || return 1
	_bashc_source_file "$bashC/shellFunctionality/shellMain.sh" || return 1
	_bashc_source_file "$bashC/standard_settings.sh" || return 1
	_bashc_source_file "$bashC/installScripts/installMain.sh" || return 1
	_bashc_source_file "$bashC/programExtensions/extentionsMain.sh" || return 1
	_bashc_source_file "$bashC/generalScripts/gScriptMain.sh" || return 1
	_bashc_source_file "$local_dir/local_main.sh" || return 1

	if [[ $1 == "" ]]; then
		echo "Done reloading files!";
	elif [[ $1 == "first_load" ]]; then
		echo "Extentions loaded!"
	fi
}

# Run 'bashc configs check' at interactive shell startup to detect config drift.
# Skipped for non-interactive shells or when BASHC_SKIP_CONFIG_CHECK is set.
#
# If 'bashc' is not on PATH (e.g. the binary was never installed), the function
# prints a warning to stderr rather than failing silently — a silent no-op hid
# real config drift during review of this feature. The same applies if 'bashc
# configs check' itself exits non-zero. Set BASHC_SKIP_CONFIG_CHECK=1 to
# suppress all output from this hook.
bashc_check_configs () {
	case $- in
		*i*) ;;
		*) return 0 ;;
	esac
	[ -n "${BASHC_SKIP_CONFIG_CHECK:-}" ] && return 0

	if ! command -v bashc >/dev/null 2>&1; then
		printf 'bashc: ⚠ bashc binary not on PATH — config drift check skipped. Install via bashCustomization/init.sh, or set BASHC_SKIP_CONFIG_CHECK=1 to silence.\n' >&2
		return 0
	fi

	bashc configs check
	rc=$?
	if [ "$rc" -ne 0 ]; then
		printf 'bashc: ⚠ bashc configs check failed (exit %s) — run it manually for details.\n' "$rc" >&2
	fi
	return 0
}
