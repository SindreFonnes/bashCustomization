#!/bin/bash
set -eo pipefail;

if [[ $bashC != "" ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    export MYINSTALL_SCRIPT_FOLDER_LOCATION="$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd )/../installScripts";
fi

export MYINSTALL_COMMON_FUNCTIONS_LOCATION=$MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;
source $MYINSTALL_COMMON_FUNCTIONS_LOCATION;

run_dedicated_install () {
	local script="$1"
	if [[ -f "$script" ]]; then
		chmod +x "$script"
		"$script" || echo "Warning: $script exited with non-zero status"
	else
		echo "Warning: Install script not found: $script"
	fi
}

# Install brew first so the dedicated install scripts can use it
run_dedicated_install "$MYINSTALL_SCRIPT_FOLDER_LOCATION/brew/installBrew.sh"

# Make brew available in this session if it was just installed
if ! command -v brew &> /dev/null; then
	if [[ -x /opt/homebrew/bin/brew ]]; then
		eval "$(/opt/homebrew/bin/brew shellenv)"
	elif [[ -x /usr/local/bin/brew ]]; then
		eval "$(/usr/local/bin/brew shellenv)"
	elif [[ -x /home/linuxbrew/.linuxbrew/bin/brew ]]; then
		eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
	fi
fi

if command -v brew &> /dev/null; then
	brew update && brew upgrade;
	brew install \
		git \
		gnupg
fi

if is_linux_os; then
	sudo apt update;

	sudo apt upgrade -y;

	sudo add-apt-repository universe -y;

	sudo apt install \
		build-essential \
		git \
		safe-rm \
		keychain \
		nala \
		gnupg \
		pkg-config \
		libssl-dev \
		zip \
		unzip \
		tar \
		gzip \
		net-tools \
		libfuse2 \
		-y;

	# Dotnet HTTPS Cert necessary package
	# https://docs.microsoft.com/en-us/aspnet/core/security/enforcing-ssl?view=aspnetcore-6.0&tabs=visual-studio#ssl-linux
	sudo apt-get install -y libnss3-tools;
fi

# Delegate to dedicated install scripts for tools that have them
run_dedicated_install "$MYINSTALL_SCRIPT_FOLDER_LOCATION/ripgrep/installRipgrep.sh"
run_dedicated_install "$MYINSTALL_SCRIPT_FOLDER_LOCATION/bat/installBat.sh"
run_dedicated_install "$MYINSTALL_SCRIPT_FOLDER_LOCATION/fd/installFd.sh"
run_dedicated_install "$MYINSTALL_SCRIPT_FOLDER_LOCATION/eza/installEza.sh"
run_dedicated_install "$MYINSTALL_SCRIPT_FOLDER_LOCATION/shellcheck/installShellcheck.sh"
