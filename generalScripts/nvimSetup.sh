#!/bin/bash

set -eo pipefail;

CONFIG_LOCATION="$HOME/.config/nvim";

install_nvim_mac () {
	brew install neovim;
}

install_nvim_apt () {
	curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim.appimage
	chmod u+x nvim.appimage
	./nvim.appimage
}

clone_nvim_chad_config () {
	if ! [ -d $CONFIG_LOCATION ]; then
		mkdir $CONFIG_LOCATION;
	fi

	rm -r "$CONFIG_LOCATION/*";

	rm -rf $HOME/.local/share/nvim;

	git clone https://github.com/NvChad/NvChad $CONFIG_LOCATION \
		--depth 1;


}

if [[ "$OSTYPE" == *"darwin"* ]]; then
	if ! command -v nvim &> /dev/null; then
    	if ! command -v brew &> /dev/null; then
		    echo "Missing brew. Please install before running this script";
			exit 1;
		fi

		install_nvim_mac;
	fi
else
	if ! command -v nvim &> /dev/null; then
		if ! command -v apt &> /dev/null; then
			echo "Non-apt and brew systems are currently not supported."
			echo "If you wish to use this script for these unsuported systems, then you will have to add support for them to this script";
			exit 1;
		fi

		install_nvim_apt;
	fi
fi

clone_nvim_chad_config;