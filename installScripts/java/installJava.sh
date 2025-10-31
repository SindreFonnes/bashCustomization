#!/bin/bash

set -eo pipefail;

name="Java";
commandAlias="java";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew install openjdk;

	script_success_message "$name";
	exit 0;
}

install_for_apt () {
	sudo apt update && 
	sudo apt install default-jre -y && 
	sudo apt install default-jdk -y &&
	script_success_message "$name";
	exit 0;
}

install_for_pacman () {
	sudo pacman -Syu --noconfirm;
	sudo pacman -S --needed --noconfirm jre-openjdk jdk-openjdk;
	
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
