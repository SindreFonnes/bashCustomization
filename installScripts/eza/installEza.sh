#!/bin/bash

set -eo pipefail;

name="eza";
commandAlias="eza"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

# Prefer brew (avoids needing to add the eza apt repository on Debian/Ubuntu)
if command -v brew &> /dev/null; then
	brew install eza &&
	script_success_message "$name";
	exit 0;
fi

if apt_package_manager_available; then
	if ! command -v gpg &> /dev/null; then
		sudo apt update;
		sudo apt install gpg -y;
	fi

	sudo mkdir -p /etc/apt/keyrings;
	curl -fsSL https://raw.githubusercontent.com/eza-community/eza/main/deb.asc | sudo gpg --dearmor -o /etc/apt/keyrings/gierens.gpg;
	echo "deb [signed-by=/etc/apt/keyrings/gierens.gpg] http://deb.gierens.de stable main" | sudo tee /etc/apt/sources.list.d/gierens.list > /dev/null;
	sudo chmod 644 /etc/apt/keyrings/gierens.gpg;
	sudo chmod 644 /etc/apt/sources.list.d/gierens.list;

	sudo apt update;
	sudo apt install eza -y;

	script_success_message "$name";
	exit 0;
fi

script_does_not_support_os "$name";

exit 1;
