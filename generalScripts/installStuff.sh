#!/bin/bash
set -eo pipefail;

if [[ $bashC != "" ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    export MYINSTALL_SCRIPT_FOLDER_LOCATION="$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd )/../installScripts";
fi

source $MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;

if is_mac_os; then
	brew update && brew upgrade;
	brew install \
		bat \
		ripgrep \
		git \
		keychain \
		gnupg
		
	exit 0;
fi

if command -v pacman &> /dev/null; then
	sudo pacman -Syu --noconfirm;
	
	sudo pacman -S --needed --noconfirm \
		base-devel \
		git \
		keychain \
		gnupg \
		pkgconf \
		openssl \
		zip \
		unzip \
		tar \
		gzip \
		ripgrep \
		net-tools \
		fuse2 \
		bat \
		nss;
	
	echo "Essential packages installed for Arch-based system!";
	exit 0;
fi

if command -v apt &> /dev/null; then
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
		ripgrep \
		net-tools \
		libfuse2 \
		-y;

	# Dotnet HTTPS Cert necessary package
	# https://docs.microsoft.com/en-us/aspnet/core/security/enforcing-ssl?view=aspnetcore-6.0&tabs=visual-studio#ssl-linux
	sudo apt-get install -y libnss3-tools;
	
	exit 0;
fi

echo "Unsupported package manager. This script supports apt (Debian/Ubuntu) and pacman (Arch).";
exit 1;
