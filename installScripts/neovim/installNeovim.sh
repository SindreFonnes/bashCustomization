#!/bin/bash

set -eo pipefail;

name="Neovim";
commandAlias="nvim";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew install neovim;

	script_success_message "$name";
	exit 0;
}

install_for_linux () {
	curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim.appimage
	chmod +x nvim.appimage
	
	if ! [[ -d $HOME/.mybin ]]; then
		mkdir $HOME/.mybin;
		echo "You need to add $HOME/.mybin to the path";
	fi

	mv ./nvim.appimage $HOME/.mybin/nvim;

	script_success_message "$name";
	exit 0;
}

if is_mac_os; then
	install_for_mac;
fi

install_for_linux;
