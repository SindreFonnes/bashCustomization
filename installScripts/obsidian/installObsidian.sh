#!/bin/bash

set -eo pipefail;

name="Obsidian";
commandAlias="obsidian";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew install --cask obsidian;

	script_success_message "$name";
	exit 0;
}

install_for_linux () {
	local NEWEST_VERSION=$(\
		curl -sl "https://github.com/obsidianmd/obsidian-releases/releases" | \
		grep -m 1 "a href=\"\/obsidianmd\/obsidian-releases\/releases\/tag\/"  | \
		grep -aom 1 v[0-9]*.[0-9]*.[0-9]* | \
		grep -oE "v[0-9]+\.[0-9]+\.[0-9]+" | \
		tr -dc '[0-9].' \
	)

	sudo apt update;
	sudo apt install -y libgbm libasound2;

	wget -P ~/ https://github.com/obsidianmd/obsidian-releases/releases/download/v${NEWEST_VERSION}/obsidian_${NEWEST_VERSION}_amd64.deb;
	sudo apt install -y ~/obsidian_${NEWEST_VERSION}_amd64.deb;

	rm ~/obsidian_${NEWEST_VERSION}_amd64.deb;

	script_success_message "$name";
}

if is_mac_os; then
	brew update &&
	brew install --cask obsidian &&
	script_success_message "$name";
	exit 0;
fi

if [[ $OSTYPE == *"linux"* ]]; then
	install_for_linux;
fi

script_does_not_support_os "$name";

exit 1;