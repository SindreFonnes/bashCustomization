#!/bin/bash

set -eo pipefail;

VERSION="0.39.1"

# export NVM_DIR=$HOME/.nvm;
# source $NVM_DIR/nvm.sh;

# Check if nvm command allready exists
if  command -v nvm &> /dev/null; then
	echo "Nvm is already installed. Please remove it if you want to reinstall it."
	exit 0;
fi

if [[ -f "./nvm-$VERSION.sh" ]]; then
	echo "Nvm script is allready downloaded";
else
	echo "Downloading nvm script";
	curl -o nvm-$VERSION.sh "https://raw.githubusercontent.com/nvm-sh/nvm/v$VERSION/install.sh"
	chmod +x ./nvm-$VERSION.sh
fi

# Run install script
./nvm-$VERSION.sh

# Remove install script
rm ./nvm-$VERSION.sh