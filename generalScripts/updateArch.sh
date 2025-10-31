#!/bin/bash

## This script is meant to update Arch-based distros (Arch, Manjaro, EndeavourOS, etc.)
set -eo pipefail;

if [[ $bashC != "" ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    export MYINSTALL_SCRIPT_FOLDER_LOCATION="$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd )/../installScripts";
fi

source $MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;

if ! is_arch_based; then
    echo "This script is only for Arch-based distributions.";
    exit 1;
fi

echo "Updating Arch-based system...";

# Update package database and upgrade all packages
sudo pacman -Syu --noconfirm;

# Clean package cache (keep last 3 versions)
sudo paccache -rk3 2>/dev/null || echo "paccache not available, skipping cache cleanup";

# Check for orphaned packages
orphans=$(pacman -Qtdq 2>/dev/null || true);
if [[ -n "$orphans" ]]; then
    echo "Found orphaned packages:";
    echo "$orphans";
    echo "Remove them? (y/n)";
    read answer;
    if [[ $answer == "y" || $answer == "Y" ]]; then
        sudo pacman -Rns --noconfirm $orphans;
    fi
fi

# Update AUR packages if AUR helper is installed
if command -v yay &> /dev/null; then
    echo "Updating AUR packages with yay...";
    yay -Syu --noconfirm;
elif command -v paru &> /dev/null; then
    echo "Updating AUR packages with paru...";
    paru -Syu --noconfirm;
fi

echo "System update complete!";
