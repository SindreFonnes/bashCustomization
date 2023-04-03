# Standard settings
PROMPT_DIRTRIM=3;

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
