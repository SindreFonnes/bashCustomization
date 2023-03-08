#!/bin/bash

set -eo pipefail

name="Brew";
commandAlias="brew";

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed $commandAlias $name; then
	exit 1;
fi

if is_wsl_os; then
	echo "Can't install $name on wsl...";
	echo "exiting...";
	exit 1;
fi

curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | sudo bash -;

script_success_message $name;

exit 0;
