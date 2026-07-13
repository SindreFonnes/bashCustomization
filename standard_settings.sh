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

_bashc_add_path_dir "/usr/local/go/bin"
_bashc_add_path_dir "$HOME/.cargo/bin"
_bashc_add_path_dir "$HOME/.local/bin"
_bashc_add_path_dir "$HOME/.mybin"
_bashc_add_path_dir "$HOME/.bun/bin"
_bashc_add_path_dir "$HOME/.local/share/pnpm"

export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
if [ -s "$NVM_DIR/nvm.sh" ]; then
	_bashc_source_file "$NVM_DIR/nvm.sh" || return 1
fi

if command -v brew >/dev/null 2>&1; then
	_bashc_openjdk_prefix=$(brew --prefix openjdk 2>/dev/null) || _bashc_openjdk_prefix=""
	if [ -n "$_bashc_openjdk_prefix" ]; then
		_bashc_add_path_dir "$_bashc_openjdk_prefix/bin"
	fi
	unset _bashc_openjdk_prefix
fi

if command -v "go" >/dev/null 2>&1; then
	export GOPATH="$HOME/p/go";
	_bashc_add_path_dir "$GOPATH/bin"
fi

## https://askubuntu.com/questions/22037/aliases-not-available-when-using-sudo
## Making sudo work with other aliases
alias sudo="sudo ";
