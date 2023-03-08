#!/bin/bash

set -eo pipefail;

if [[ $SHELL != "/bin/bash" ]]; then
	echo "You are allready not running bash";
	exit 0;
fi

sudo apt update;

sudo apt install git zsh -y;

sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)";