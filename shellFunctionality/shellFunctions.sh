# shellcheck shell=bash
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
	local current_location
	local command_status
	current_location=$(pwd) || return 1
	cd "$2" || return 1
	"$1"
	command_status=$?
	cd "$current_location" || return 1

	if [[ $command_status -ne 0 ]]; then
		return "$command_status"
	fi

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
_bashc_change_variable_case () {
	local variable_name=$1
	local source_characters=$2
	local target_characters=$3
	local variable_value

	case "$variable_name" in
		''|[!a-zA-Z_]*|*[!a-zA-Z0-9_]*)
			printf 'bashc: invalid variable name: %s\n' "$variable_name" >&2
			return 1
			;;
	esac

	# The identifier is validated before indirection, so eval cannot introduce
	# syntax. printf -v works in the macOS system Bash as well as Zsh, unlike
	# Bash namerefs (`local -n`), which require a newer Bash.
	eval "variable_value=\"\${$variable_name}\""
	variable_value=$(printf '%s' "$variable_value" | tr "$source_characters" "$target_characters") || return 1
	printf -v "$variable_name" '%s' "$variable_value"
}

variable_to_lowercase () {
	_bashc_change_variable_case "$1" '[:upper:]' '[:lower:]'
}

variable_to_uppercase () {
	_bashc_change_variable_case "$1" '[:lower:]' '[:upper:]'
}

pushd_wrapper () {
	if [[ $# -eq 0 ]]; then
		pushd ~ &> /dev/null || return 1;
	else
		pushd "$1" &> /dev/null || return 1;
	fi
}

popd_wrapper () {
	popd &> /dev/null || return 1;
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
	find . ! -name . -prune -exec du -sh {} \;
}

get_all_files_bellow_directory () {
	local start_dir="$1";

	if [[ $start_dir == "" ]]; then
		start_dir=$(pwd);
	fi

	find "$start_dir" -type f -print | while IFS= read -r entry; do
		printf '%s\n' "${entry#"$start_dir"/}"
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

# -----------------------------------------------------------------------------
# Zellij session/project switcher
#
# Dependencies:
#   zellij
#   fd
#   fzf
#
# Works in both Bash and zsh.
# -----------------------------------------------------------------------------

_zellij_projects() {
    fd . "$HOME/p" "$HOME/p/scaleaq" \
        --min-depth 1 \
        --max-depth 1 \
        --type directory \
        2>/dev/null
}


_zellij_sessions() {
    zellij list-sessions --no-formatting 2>/dev/null |
        awk '
            NF && $0 !~ /EXITED/ {
                print $1
            }
        '
}


_zellij_session_exists() {
    _zellij_sessions | grep -Fxq -- "$1"
}


_zellij_inside() {
    [ -n "${ZELLIJ:-}" ] || [ -n "${ZELLIJ_SESSION_NAME:-}" ]
}


_zellij_switch_to_session() {
    local session_name="$1"

    # Already in the requested session.
    if [ "${ZELLIJ_SESSION_NAME:-}" = "$session_name" ]; then
        return 0
    fi

    if _zellij_inside; then
        zellij action switch-session "$session_name"
    else
        zellij attach "$session_name"
    fi
}


_zellij_resolve_project() {
    local arg="$1"

    # Explicit/relative/absolute path.
    if [ -d "$arg" ]; then
        (
            cd "$arg" 2>/dev/null || exit 1
            pwd -P
        )
        return
    fi

    # Project directly below ~/p.
    if [ -d "$HOME/p/$arg" ]; then
        (
            cd "$HOME/p/$arg" 2>/dev/null || exit 1
            pwd -P
        )
        return
    fi

    # Project below ~/p/scaleaq.
    if [ -d "$HOME/p/scaleaq/$arg" ]; then
        (
            cd "$HOME/p/scaleaq/$arg" 2>/dev/null || exit 1
            pwd -P
        )
        return
    fi

    return 1
}


_zellij_fzf_target() {
    {
        # Existing sessions
        _zellij_sessions |
            while IFS= read -r session; do
                [ -n "$session" ] &&
                    printf 'session\t%s\n' "$session"
            done

        # Projects
        _zellij_projects |
            while IFS= read -r project; do
                [ -n "$project" ] &&
                    printf 'project\t%s\n' "${project%/}"
            done
    } |
        fzf \
            --delimiter=$'\t' \
            --with-nth=1,2 \
            --bind 'tab:down' \
            --bind 'btab:up'
}


swap_zellij_session() {
    local selected
    local selected_type
    local selected_value
    local selected_name
    local arg
    local layout="compact"

    case "$#" in
        0)
            selected="$(_zellij_fzf_target)"

            [ -z "$selected" ] && return 0

            selected_type="${selected%%$'\t'*}"
            selected_value="${selected#*$'\t'}"

            if [ "$selected_type" = "session" ]; then
                _zellij_switch_to_session "$selected_value"
                return
            fi

            selected="$selected_value"
            ;;

        1)
            arg="$1"

            # Existing session names take precedence over project names.
            if _zellij_session_exists "$arg"; then
                _zellij_switch_to_session "$arg"
                return
            fi

            selected="$(_zellij_resolve_project "$arg")"

            if [ -z "$selected" ]; then
                printf 'No session or project found: %s\n' "$arg" >&2
                return 1
            fi
            ;;

        *)
            printf 'Usage: swap_zellij_session [session|project|path]\n' >&2
            return 2
            ;;
    esac

    # Canonicalize project path.
    selected="$(
        cd "$selected" 2>/dev/null &&
            pwd -P
    )"

    if [ -z "$selected" ]; then
        printf 'Invalid project directory\n' >&2
        return 1
    fi

    selected_name="$(
        basename "$selected" |
            tr '.' '_'
    )"

    # A session for this project already exists.
    # Switch to it without modifying its cwd/layout.
    if _zellij_session_exists "$selected_name"; then
        _zellij_switch_to_session "$selected_name"
        return
    fi

    # No existing session: create one for the project.
    if _zellij_inside; then
        zellij action switch-session \
            "$selected_name" \
            --cwd "$selected" \
            --layout "$layout"
    else
        # Use a subshell so we don't change the caller's cwd.
        (
            cd "$selected" || exit 1

            zellij attach --create "$selected_name" \
                options --default-layout "$layout"
        )
    fi
}


# -----------------------------------------------------------------------------
# Completion candidates shared between Bash and zsh
# -----------------------------------------------------------------------------

_zellij_completion_candidates() {
    {
        _zellij_sessions

        _zellij_projects |
            while IFS= read -r project; do
                project="${project%/}"
                basename "$project"
            done
    } |
        awk '
            NF && !seen[$0]++ {
                print
            }
        '
}


# -----------------------------------------------------------------------------
# Bash completion
# -----------------------------------------------------------------------------

_swap_zellij_session_bash_completion() {
    local cur
    local candidate
    local directory
    local cur_lower
    local candidate_lower

    cur="${COMP_WORDS[COMP_CWORD]}"
    cur_lower=$(printf '%s' "$cur" | tr '[:upper:]' '[:lower:]')

    COMPREPLY=()

    # Sessions + known project names, case-insensitive.
    while IFS= read -r candidate; do
        candidate_lower=$(printf '%s' "$candidate" | tr '[:upper:]' '[:lower:]')

        case "$candidate_lower" in
            "$cur_lower"*)
                COMPREPLY+=("$candidate")
                ;;
        esac
    done < <(_zellij_completion_candidates)

    # Normal directory/path completion.
    while IFS= read -r directory; do
        COMPREPLY+=("$directory")
    done < <(compgen -d -- "$cur")

    compopt -o filenames 2>/dev/null || true
}

# -----------------------------------------------------------------------------
# zsh completion
# -----------------------------------------------------------------------------

_swap_zellij_session_zsh_completion() {
    local candidate
    local -a candidates

    candidates=()

    while IFS= read -r candidate; do
        candidates+=("$candidate")
    done < <(_zellij_completion_candidates)

    # Sessions/projects, case-insensitive.
    compadd -M 'm:{a-zA-Z}={A-Za-z}' -- "${candidates[@]}"

    # Also retain normal directory/path completion.
    _directories
}


# -----------------------------------------------------------------------------
# Register completion for whichever shell sourced this file
# -----------------------------------------------------------------------------

if [ -n "${BASH_VERSION:-}" ]; then
    complete -F _swap_zellij_session_bash_completion swap_zellij_session

elif [ -n "${ZSH_VERSION:-}" ]; then
    # compdef is provided by zsh's completion system.
    # Initialize it only if the user's .zshrc hasn't already done so.
    if ! command -v compdef >/dev/null 2>&1; then
        autoload -Uz compinit
        compinit
    fi

    compdef _swap_zellij_session_zsh_completion swap_zellij_session
fi
