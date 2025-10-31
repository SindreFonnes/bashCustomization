#!/bin/bash

set -eo pipefail;

name="Github CLI";
commandAlias="gh"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew install gh &&
	script_success_message "$name";
	exit 0;
}

install_for_apt () {
	curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg;

	sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg;

	echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null;

	sudo apt update;

	sudo apt install gh -y;

	script_success_message "$name";
	exit 0;
}

install_for_pacman () {
	sudo pacman -Syu --noconfirm;
	sudo pacman -S --needed --noconfirm github-cli;
	
	script_success_message "$name";
	exit 0;
}

if is_mac_os; then
	install_for_mac;
fi

if apt_package_manager_available; then
	install_for_apt;
fi

if pacman_package_manager_available; then
	install_for_pacman;
fi

script_does_not_support_os "$name";

exit 1;
