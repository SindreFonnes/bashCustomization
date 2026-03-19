#!/bin/bash

set -eo pipefail;

name="fd";
commandAlias="fd"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

# On Debian/Ubuntu apt installs as 'fdfind' — check both names
if command -v fd &> /dev/null || command -v fdfind &> /dev/null; then
	script_allready_installed "$name";
	exit 1;
fi

# Prefer brew (avoids the fdfind naming issue on Debian/Ubuntu)
if command -v brew &> /dev/null; then
	brew install fd &&
	script_success_message "$name";
	exit 0;
fi

if apt_package_manager_available; then
	sudo apt update;
	sudo apt install fd-find -y;

	# On Debian/Ubuntu, fd is installed as 'fdfind' — create a symlink so 'fd' works
	if command -v fdfind &> /dev/null && ! command -v fd &> /dev/null; then
		local bin_dir="$HOME/.local/bin"
		mkdir -p "$bin_dir"
		ln -sf "$(command -v fdfind)" "$bin_dir/fd"
		echo "Note: Created symlink $bin_dir/fd -> fdfind"
		echo "Ensure $bin_dir is in your PATH"
	fi

	script_success_message "$name";
	exit 0;
fi

script_does_not_support_os "$name";

exit 1;
