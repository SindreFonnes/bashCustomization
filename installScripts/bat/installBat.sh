#!/bin/bash

set -eo pipefail;

name="bat";
commandAlias="bat"

source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

# On Debian/Ubuntu apt installs as 'batcat' — check both names
if command -v bat &> /dev/null || command -v batcat &> /dev/null; then
	script_allready_installed "$name";
	exit 1;
fi

# Prefer brew (avoids the batcat naming issue on Debian/Ubuntu)
if command -v brew &> /dev/null; then
	brew install bat &&
	script_success_message "$name";
	exit 0;
fi

if apt_package_manager_available; then
	sudo apt update;
	sudo apt install bat -y;

	# On Debian/Ubuntu, bat is installed as 'batcat' — create a symlink so 'bat' works
	if command -v batcat &> /dev/null && ! command -v bat &> /dev/null; then
		local bin_dir="$HOME/.local/bin"
		mkdir -p "$bin_dir"
		ln -sf "$(command -v batcat)" "$bin_dir/bat"
		echo "Note: Created symlink $bin_dir/bat -> batcat"
		echo "Ensure $bin_dir is in your PATH"
	fi

	script_success_message "$name";
	exit 0;
fi

script_does_not_support_os "$name";

exit 1;
