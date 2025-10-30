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

# Install Homebrew (NEVER use sudo - Homebrew explicitly warns against this)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Evaluate shellenv to make brew available in current session
if [[ -x /opt/homebrew/bin/brew ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
elif [[ -x /usr/local/bin/brew ]]; then
    eval "$(/usr/local/bin/brew shellenv)"
fi

script_success_message $name;

exit 0;
