#!/bin/bash

set -eo pipefail

name="Kubectl";
commandAlias="kubectl";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed $commandAlias $name; then
	exit 1;
fi

install_for_mac () {
	brew update &&
	brew install \
		kubernetes-cli \
		kubectx &&
	script_success_message "$name";
	exit 0;
}

install_for_linux () {
	curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl";
	curl -LO "https://dl.k8s.io/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl.sha256";
	CHECK_RESULT=$(echo "$(cat kubectl.sha256)  kubectl" | sha256sum --check)

	if [[ $CHECK_RESULT == *"OK"* ]]; then
		echo "Checksum OK";
		
		sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl;

		rm ./kubectl;
		rm ./kubectl.sha256;

		script_success_message "$name";
		exit 0;
	else
		echo "Checksum NOT OK";
		rm ./kubectl;
		rm ./kubectl.sha256;
		exit 1;
	fi

	script_success_message ""
}

if is_mac_os; then
	install_for_mac;
fi

if [[ $OSTYPE == *"linux"* ]]; then
	install_for_linux;
fi

script_does_not_support_os "$name";

exit 1;
