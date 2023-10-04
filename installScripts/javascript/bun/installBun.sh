#!/bin/bash

set -eo pipefail;

VERSION="0.39.1"

# export NVM_DIR=$HOME/.nvm;
# source $NVM_DIR/nvm.sh;

# Check if nvm command allready exists
if  command -v bun &> /dev/null; then
	echo "Bun is already installed. Please remove it if you want to reinstall it."
	exit 0;
fi

curl -fsSL https://bun.sh/install | bash
