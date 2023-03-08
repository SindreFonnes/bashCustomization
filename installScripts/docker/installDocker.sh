#!/bin/bash

set -eo pipefail;

name="Docker";
commandAlias="docker";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew cask install docker &&
	script_success_message "$name";
	exit 0;
}

install_for_apt () {
	sudo apt update;

	# Make sure all libraries needed to handle gpg keys is installed
	sudo apt install \
		ca-certificates \
		curl \
		gnupg \
		lsb-release \
		-y

	# Add docker gpg key
	sudo mkdir -p /etc/apt/keyrings
	curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

	# Add docker repo
	echo \
	"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
	$(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

	sudo apt update;

	if [[ $? -ne 0 ]]; then
		sudo chmod a+r /etc/apt/keyrings/docker.gpg;
	fi

	sudo apt install \
		docker-ce \
		docker-ce-cli \
		containerd.io \
		docker-compose-plugin \
		-y
	
	echo "Docker is now installed!";

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
