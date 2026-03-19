#!/bin/bash

set -eo pipefail;

name="ripgrep";
commandAlias="rg"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

if command -v brew &> /dev/null; then
	brew install ripgrep &&
	script_success_message "$name";
	exit 0;
fi

if apt_package_manager_available; then
	sudo apt update;
	sudo apt install ripgrep -y;
	script_success_message "$name";
	exit 0;
fi

script_does_not_support_os "$name";

exit 1;
