#!/bin/bash

set -eo pipefail

name="Yarn";
commandAlias="yarn";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed $commandAlias $name; then
	exit 1;
fi

install_for_mac () {
    brew update &&
    brew install yarn;
    script_success_message "$name";
	exit 0;
}

install_for_apt () {
    curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | sudo apt-key add -;
    echo "deb https://dl.yarnpkg.com/debian/ stable main" | sudo tee /etc/apt/sources.list.d/yarn.list;
    sudo apt update && sudo apt install yarn -y;
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
