#!/bin/bash

set -eo pipefail;

name="Rust";
commandAlias="rustc"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

if ! script_check_if_allready_installed "$commandAlias" "$name"; then
	exit 1;
fi

if [[ -f "./rustup-init.sh" ]]; then
	echo "Rustup init script is allready downloaded";
else
	echo "Downloading rustup init script";
	curl --proto '=https' --tlsv1.2 -sSf -o rustup-init.sh -L https://sh.rustup.rs
	chmod +x rustup-init.sh
fi

# Run install script
./rustup-init.sh -y

# Remove install script
rm ./rustup-init.sh