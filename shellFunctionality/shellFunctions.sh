# Functions

## General utility
restart_shell () {
	if  [[ $PROFILE_SHELL == zsh ]]; then
		exec zsh -l;
	elif [[ $PROFILE_SHELL == bash ]]; then
		exec bash -l;
	else
		echo "Uknown shell. Modify function and add your shell if you want to use this function.";
	fi
}

execute_command_in_folder_and_go_back () {
	local current_location=$(pwd) &&
	cd "$2" &&
	$1;
	cd "$current_location";
	echo "Done";
}

start_or_install_keychain () {
	local no_key_agent="Could not open a connection to your authentication agent.";
	local error_connecting="Error connecting to agent: No such file or directory";
	local error_no_id="The agent has no identities."

	if [[ -f ~/.ssh/id_ed25519 ]]; then
		local key_to_use=id_ed25519;
	elif [[ -f ~/.ssh/id_rsa ]]; then
		local key_to_use=id_rsa;
	fi

	if command -v keychain &> /dev/null; then
		local keychain_agents="$(keychain -l 2>&1)";
		
		if [[ $key_to_use == id_ed25519 || $key_to_use == id_rsa ]] && 
		[[ "$keychain_agents" == "$no_key_agent" || "$keychain_agents" == "$error_connecting" || "$keychain_agents" == "$error_no_id" ]]; then
			eval $(keychain --agents ssh --eval $key_to_use --clear);
		fi
		return 0;
	fi
	
	local INSTALLED_KEYCHAIN="Installed keychain, restart shell to run it"

	echo "Installing keychain, restart shell for it to load potential ssh key"
	
	if [[ $IS_MAC == "true" ]]; then
    	brew update && brew install keychain && echo "$INSTALLED_KEYCHAIN";
	else 
		sudo apt update && sudo apt install keychain && echo "$INSTALLED_KEYCHAIN";
	fi
}

update_packages () {
	if [[ $IS_MAC == "true" ]]; then
		echo "Updating brew packages..."
		brew update && brew upgrade;
	else
		echo "Updating apt packages..."
		sudo apt update && sudo apt upgrade -y;
	fi
}

# Takes the NAME, not the actual variable, of a variable as an argument and changes the string in the variable to be lowercase
variable_to_lowercase () {
	local -n variable_to_modify=$1;
	variable_to_modify=$(echo "$variable_to_modify" | tr '[:upper:]' '[:lower:]');
	return;
}

variable_to_uppercase () {
	local -n variable_to_modify=$1;
	variable_to_modify=$(echo "$variable_to_modify" | tr '[:lower:]' '[:upper:]');
	return;
}

pushd_wrapper () {
	if [[ $# -eq 0 ]]; then
		pushd ~ &> /dev/null;
	else
		pushd "$1" &> /dev/null;
	fi
}

popd_wrapper () {
	popd &> /dev/null;
}

grep_specific_filetype_in_subfolders () {
	grep -inr --include "$1" "$2";
}

find_entity_size () {
	if [[ $? -eq 0 ]]; then
		find_all_items_in_folder_size;
		return;
	fi
	
	du -sh "$1";
}

find_all_items_in_folder_size () {
	la -1 | du -sh $(</dev/stdin);
}

get_all_files_bellow_directory () {
	local all_files=()

	local start_dir="$1";

	if [[ $start_dir == "" ]]; then
		start_dir=$(pwd);
	fi
	
	for entry in $(ls -p $start_dir); do
		if [[ $entry == *"/"* ]]; then
			for subEntry in $(get_all_files_bellow_directory $start_dir/$entry); do
				all_files+=("$entry$subEntry");
			done;
		else
			all_files+=($entry);
		fi
	done

	for entry in ${all_files[@]}; do
		echo $entry;
	done
}
