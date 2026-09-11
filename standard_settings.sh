# shellcheck shell=bash
# Standard settings
PROMPT_DIRTRIM=3;

# https://phoenixnap.com/kb/change-bash-prompt-linux // some ways to customize it
# https://gist.github.com/JBlond/2fea43a3049b38287e5e9cefc87b2124 // Ansi color table



if [[ $PROFILE_SHELL == "zsh" && -n "$ZSH" && -f "$ZSH/oh-my-zsh.sh" ]]; then
    # Consumed by oh-my-zsh after this sourced settings file returns.
    # shellcheck disable=SC2034
	plugins=(
		git
		colored-man-pages
		common-aliases
		command-not-found
		copybuffer
		copyfile
		copypath
		dirhistory
		docker
		docker-compose
		extract
		git-prompt
		golang
		helm
		history-substring-search
		screen
		vscode
		zsh-interactive-cd
		zsh-navigation-tools
	)
		_bashc_source_file "$ZSH/oh-my-zsh.sh" || return 1
fi

if [[ $IS_MAC == "true" ]]; then
	unset NODE_OPTIONS;
else
	echo "";
	# PS1="\[\e]0;\u@\h: \w\a\]${debian_chroot:+($debian_chroot)}\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\\n\e[0;32m> \e[0m"
fi

# Add 
# ssh-add;

# Exports
_bashc_add_path_dir () {
	[ -d "$1" ] || return 0
	case ":$PATH:" in
		*":$1:"*) ;;
		*) PATH="$PATH:$1" ;;
	esac
	export PATH
}

_bashc_prepend_path_dir () {
	[ -d "$1" ] || return 0
	local remaining_path="$PATH"
	local path_entry filtered_path="" separator="" last_entry
	# Move an existing entry to the front too. Preserve spaces and empty PATH
	# components without relying on Bash/Zsh-specific array splitting.
	while :; do
		last_entry=false
		case "$remaining_path" in
			*:*) path_entry=${remaining_path%%:*}; remaining_path=${remaining_path#*:} ;;
			*) path_entry=$remaining_path; last_entry=true ;;
		esac
		if [ "$path_entry" != "$1" ]; then
			filtered_path="$filtered_path$separator$path_entry"
			separator=:
		fi
		[ "$last_entry" = true ] && break
	done
	PATH="$1$separator$filtered_path"
	export PATH
}

_bashc_activate_homebrew () {
	local brew_program brew_prefix
	brew_program=$(command -v brew 2>/dev/null) || brew_program=""
	if [ -z "$brew_program" ] && [ -n "${HOMEBREW_PREFIX:-}" ] && [ -x "$HOMEBREW_PREFIX/bin/brew" ]; then
		brew_program="$HOMEBREW_PREFIX/bin/brew"
	fi
	if [ -z "$brew_program" ]; then
		if [ "$IS_MAC" = true ]; then
			for brew_program in /opt/homebrew/bin/brew /usr/local/bin/brew; do
				[ -x "$brew_program" ] && break
			done
		else
			brew_program=/home/linuxbrew/.linuxbrew/bin/brew
		fi
		[ -x "$brew_program" ] || return 0
	fi
	brew_prefix=$("$brew_program" --prefix 2>/dev/null) || return 0
	case "$brew_prefix" in
		/*) ;;
		*) return 0 ;;
	esac
	export HOMEBREW_PREFIX="$brew_prefix"
	_bashc_prepend_path_dir "$brew_prefix/sbin"
	_bashc_prepend_path_dir "$brew_prefix/bin"
}

# The Rust installer cannot update the environment of this parent/future shell.
_bashc_activate_homebrew

_bashc_install_dir=${BASHC_INSTALL_DIR:-}
if [[ -z "$_bashc_install_dir" && -r "$HOME/.config/bashc/install_dir" ]]; then
	IFS= read -r _bashc_install_dir < "$HOME/.config/bashc/install_dir" || _bashc_install_dir=""
fi
if [[ -z "$_bashc_install_dir" ]]; then
	_bashc_install_dir="$HOME/.mybin"
fi
export BASHC_INSTALL_DIR="$_bashc_install_dir"
_bashc_prepend_path_dir "$BASHC_INSTALL_DIR"
unset _bashc_install_dir

_bashc_add_path_dir "/usr/local/go/bin"
_bashc_add_path_dir "$HOME/.cargo/bin"
_bashc_add_path_dir "$HOME/.local/bin"
_bashc_add_path_dir "$HOME/.bun/bin"
_bashc_add_path_dir "$HOME/.local/share/pnpm"

export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
if [ -s "$NVM_DIR/nvm.sh" ]; then
	_bashc_source_file "$NVM_DIR/nvm.sh" || return 1
fi

if command -v brew >/dev/null 2>&1; then
	for _bashc_formula in openjdk rustup; do
		_bashc_formula_prefix=$(brew --prefix "$_bashc_formula" 2>/dev/null) || _bashc_formula_prefix=""
		if [ -n "$_bashc_formula_prefix" ]; then
			if [ "$_bashc_formula" = openjdk ]; then
				_bashc_prepend_path_dir "$_bashc_formula_prefix/bin"
			else
				_bashc_add_path_dir "$_bashc_formula_prefix/bin"
			fi
		fi
	done
	unset _bashc_formula _bashc_formula_prefix
fi

if command -v "go" >/dev/null 2>&1; then
	export GOPATH="$HOME/p/go";
	_bashc_add_path_dir "$GOPATH/bin"
fi

## https://askubuntu.com/questions/22037/aliases-not-available-when-using-sudo
## Making sudo work with other aliases
alias sudo="sudo ";
