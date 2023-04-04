#!/bin/bash

set -eo pipefail;

CONFIG_LOCATION="$HOME/.config/nvim";

clone_nvim_chad_config () {
	if ! [ -d "$HOME/.config" ]; then
		mkdir "$HOME/.config";
	fi

	if [ -d $CONFIG_LOCATION ]; then
		rm -rf $CONFIG_LOCATION;
	fi

	rm -rf $HOME/.local/share/nvim;

	git clone https://github.com/NvChad/NvChad $CONFIG_LOCATION \
		--depth 1;

	nvim;

	nvim --headless +"TSInstall javascript"

}

run_my_install neovim -y

clone_nvim_chad_config;