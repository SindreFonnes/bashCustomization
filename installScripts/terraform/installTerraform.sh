#!/bin/bash

set -eo pipefail;

name="Terraform";
commandAlias="terraform"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew install terraform &&
	script_success_message "$name";
	exit 0;
}

install_for_apt () {
	sudo apt update;
	wget -O- https://apt.releases.hashicorp.com/gpg | \
		gpg --dearmor | sudo tee /usr/share/keyrings/hashicorp-archive-keyring.gpg;

	echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | \
		sudo tee /etc/apt/sources.list.d/hashicorp.list;

	sudo apt update && sudo apt install terraform

	script_success_message "$name";
	exit 0;
}

if is_mac_os; then
	install_for_mac;
fi

if apt_package_manager_available; then
	install_for_apt;
fi

script_does_not_support_os "$name";

exit 1;
