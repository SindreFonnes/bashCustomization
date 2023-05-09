# Standard settings
PROMPT_DIRTRIM=3;

# https://phoenixnap.com/kb/change-bash-prompt-linux // some ways to customize it
# https://gist.github.com/JBlond/2fea43a3049b38287e5e9cefc87b2124 // Ansi color table



if [[ $SHELL == "/usr/bin/zsh" ]]; then
	#if ! [ -d ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/fzf-zsh-plugin ]; then
	#	git clone https://github.com/unixorn/fzf-zsh-plugin.git ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/fzf-zsh-plugin;
	#fi

	plugins=(git \
		colored-man-pages \
		common-aliases \
		command-not-found \
		copybuffer \
		copyfile \
		copypath \
		dirhistory \
		docker \
		docker-compose \
		extract \
		git-prompt \
		golang \
		helm \
		history-substring-search \
		screen \
		vscode \
		zsh-interactive-cd \
		# fzf-zsh-plugin \
		zsh-navigation-tools \
	);
	source $ZSH/oh-my-zsh.sh;
fi

if [[ $IS_MAC == "true" ]]; then
	unset NODE_OPTIONS;
else
	PS1="\[\e]0;\u@\h: \w\a\]${debian_chroot:+($debian_chroot)}\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\\n\e[0;32m> \e[0m"
fi

# Add 
# ssh-add;

# Exports
if command -v "go" &> /dev/null; then
	export PATH=$PATH:/usr/local/go/bin;
	export GOPATH=$HOME/p/go;
	export PATH=$PATH:$GOPATH/bin;
fi

if [ -d $HOME/.mybin ]; then
	export PATH=$PATH:$HOME/.mybin;
fi
