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

ensure_ssh_agent () {
	# macOS: ssh-agent is managed by launchd automatically.
	# Just ensure ~/.ssh/config has UseKeychain + AddKeysToAgent so passphrases
	# persist across reboots without any per-operation work.
	if [[ $IS_MAC == "true" ]]; then
		_ensure_macos_ssh_config
		return 0
	fi

	# WSL / headless Linux: use keychain to persist agent across sessions
	if [[ $IS_WSL == "true" ]] || ! _has_systemd_ssh_agent; then
		_ensure_keychain
		return $?
	fi

	# Linux with systemd: use the systemd ssh-agent user service
	_ensure_systemd_ssh_agent
}

_ensure_macos_ssh_config () {
	local ssh_config="$HOME/.ssh/config"

	# Nothing to do if already configured
	if [[ -f "$ssh_config" ]] && grep -q "UseKeychain" "$ssh_config" 2>/dev/null; then
		return 0
	fi

	# Determine which key exists
	local key_name=""
	if [[ -f "$HOME/.ssh/id_ed25519" ]]; then
		key_name="id_ed25519"
	elif [[ -f "$HOME/.ssh/id_rsa" ]]; then
		key_name="id_rsa"
	else
		return 0
	fi

	mkdir -p "$HOME/.ssh"
	chmod 700 "$HOME/.ssh"

	# Prepend the Host * block (preserves any existing config below)
	# Uses ~ for IdentityFile since SSH interprets it natively
	local config_block
	config_block="Host *
    AddKeysToAgent yes
    UseKeychain yes
    IdentityFile ~/.ssh/$key_name
"
	if [[ -f "$ssh_config" ]]; then
		local existing
		existing=$(cat "$ssh_config")
		printf '%s\n\n%s\n' "$config_block" "$existing" > "$ssh_config"
	else
		printf '%s\n' "$config_block" > "$ssh_config"
	fi
	chmod 600 "$ssh_config"

	# Add key to macOS keychain (one-time, passphrase will be prompted)
	ssh-add --apple-use-keychain "$HOME/.ssh/$key_name" 2>/dev/null
	echo "Configured macOS SSH agent with UseKeychain for $key_name"
}

_has_systemd_ssh_agent () {
	# Check if systemd user services are available and ssh-agent.service exists
	command -v systemctl &>/dev/null &&
	systemctl --user cat ssh-agent.service &>/dev/null 2>&1
}

_ensure_systemd_ssh_agent () {
	# Enable the systemd user ssh-agent service if not already running
	if ! systemctl --user is-active --quiet ssh-agent.service 2>/dev/null; then
		systemctl --user enable --now ssh-agent.service 2>/dev/null
	fi

	# Point SSH_AUTH_SOCK to the systemd socket if not already set
	if [[ -z "$SSH_AUTH_SOCK" || ! -S "$SSH_AUTH_SOCK" ]]; then
		export SSH_AUTH_SOCK="${XDG_RUNTIME_DIR}/ssh-agent.socket"
	fi

	# Ensure AddKeysToAgent is set so keys are loaded on first use
	local ssh_config="$HOME/.ssh/config"
	if [[ ! -f "$ssh_config" ]] || ! grep -q "AddKeysToAgent" "$ssh_config" 2>/dev/null; then
		mkdir -p "$HOME/.ssh"
		chmod 700 "$HOME/.ssh"
		printf 'Host *\n    AddKeysToAgent yes\n\n' >> "$ssh_config"
		chmod 600 "$ssh_config"
	fi
}

_ensure_keychain () {
	local key_to_use=""
	if [[ -f ~/.ssh/id_ed25519 ]]; then
		key_to_use=id_ed25519
	elif [[ -f ~/.ssh/id_rsa ]]; then
		key_to_use=id_rsa
	fi

	if command -v keychain &>/dev/null; then
		if [[ -n "$key_to_use" ]]; then
			local keychain_status
			keychain_status="$(keychain -l 2>&1)"

			local no_agent="Could not open a connection to your authentication agent."
			local err_connect="Error connecting to agent: No such file or directory"
			local no_id="The agent has no identities."

			if [[ "$keychain_status" == "$no_agent" || "$keychain_status" == "$err_connect" || "$keychain_status" == "$no_id" ]]; then
				eval "$(keychain --agents ssh --eval "$key_to_use" --clear)"
			fi
		fi
		return 0
	fi

	echo "Installing keychain (needed for SSH agent on this platform)..."
	if [[ $IS_MAC == "true" ]]; then
		brew update && brew install keychain
	else
		sudo apt update && sudo apt install -y keychain
	fi
	echo "Installed keychain, restart shell to activate"
}

# Backwards-compatible alias for callers that still reference the old name
start_or_install_keychain () {
	ensure_ssh_agent
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
	if [[ $PROFILE_SHELL == "bash" ]]; then
		local -n variable_to_modify=$1;
		variable_to_modify=$(echo "$variable_to_modify" | tr '[:upper:]' '[:lower:]');
	else
		# zsh does not support local -n (namerefs), use eval as a portable fallback
		eval "$1=\"\$(echo \"\${$1}\" | tr '[:upper:]' '[:lower:]')\""
	fi
}

variable_to_uppercase () {
	if [[ $PROFILE_SHELL == "bash" ]]; then
		local -n variable_to_modify=$1;
		variable_to_modify=$(echo "$variable_to_modify" | tr '[:lower:]' '[:upper:]');
	else
		eval "$1=\"\$(echo \"\${$1}\" | tr '[:lower:]' '[:upper:]')\""
	fi
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
	if [[ $# -eq 0 ]]; then
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

# This function automatically detects the operating system and display server
# to use the correct command for copying piped input to the system clipboard.
# It supports macOS, Windows Subsystem for Linux (WSL), and Linux with
# either X11 or Wayland.
#
# Usage:
#   echo "Hello, clipboard!" | pb
#   cat my_file.txt | pb
#   ls -la | pb
output_to_clipboad() {
    # Check if running on macOS
    if [[ "$(uname)" == "Darwin" ]]; then
        # On macOS, pbcopy is the standard command to copy to the clipboard.
        pbcopy
    # Check if running on WSL (Windows Subsystem for Linux)
    elif grep -qE "(Microsoft|WSL)" /proc/version &> /dev/null; then
        # On WSL, we can interface with the Windows clipboard via clip.exe.
        clip.exe
    # Check if running on Linux
    elif [[ "$(uname)" == "Linux" ]]; then
        # On Linux, the clipboard utility depends on the display server.
        # We check for Wayland first. The $WAYLAND_DISPLAY variable is a reliable indicator.
        if [[ -n "$WAYLAND_DISPLAY" ]]; then
            # On Wayland, wl-copy is the standard.
            # Check if wl-copy is installed.
            if command -v wl-copy &> /dev/null; then
                wl-copy
            else
                echo "Error: wl-copy is not installed. Please install it to use the clipboard on Wayland." >&2
                return 1
            fi
        # If not Wayland, we assume X11 (X.Org).
        # The $DISPLAY variable is a reliable indicator for an X session.
        elif [[ -n "$DISPLAY" ]]; then
            # On X11, xclip is a common tool.
            # Check if xclip is installed.
            if command -v xclip &> /dev/null; then
                xclip -selection clipboard
            else
                echo "Error: xclip is not installed. Please install it to use the clipboard on X11." >&2
                return 1
            fi
        else
            echo "Error: Could not determine display server (Wayland or X11)." >&2
            echo "Cannot copy to clipboard." >&2
            return 1
        fi
    else
        echo "Error: Unsupported operating system." >&2
        return 1
    fi
}
