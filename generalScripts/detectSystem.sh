#!/bin/bash

## This script detects the current system and provides relevant information
set -eo pipefail;

if [[ $bashC != "" ]]; then
    export MYINSTALL_SCRIPT_FOLDER_LOCATION=$bashC/installScripts;
else
    export MYINSTALL_SCRIPT_FOLDER_LOCATION="$( cd -- "$( dirname -- "$BASH_SOURCE" )" &> /dev/null && pwd )/../installScripts";
fi

source $MYINSTALL_SCRIPT_FOLDER_LOCATION/commonMyinstallFunctions.sh;

echo "=== System Detection ==="
echo ""

# Detect OS Type
if is_mac_os; then
    echo "Operating System: macOS"
    BREW_PREFIX=$(brew --prefix 2>/dev/null || echo "not installed")
    echo "Homebrew: $BREW_PREFIX"
    echo "Package Manager: brew"
    echo ""
    echo "Update command: brew update && brew upgrade"
    echo "Install command: brew install <package>"
    
elif is_arch_based; then
    echo "Operating System: Arch-based Linux"
    
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        echo "Distribution: $NAME"
        echo "Version: $VERSION"
    fi
    
    echo "Package Manager: pacman"
    
    if command -v yay &> /dev/null; then
        echo "AUR Helper: yay"
    elif command -v paru &> /dev/null; then
        echo "AUR Helper: paru"
    else
        echo "AUR Helper: not installed (will be installed automatically when needed)"
    fi
    
    echo ""
    echo "Update command: sudo pacman -Syu"
    echo "Install command: sudo pacman -S <package>"
    echo "AUR install: yay -S <package>"
    echo "Script command: gscript updateArch"
    
elif is_ubuntu_debian; then
    echo "Operating System: Debian-based Linux"
    
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        echo "Distribution: $NAME"
        echo "Version: $VERSION"
    fi
    
    echo "Package Manager: apt"
    
    if is_wsl_os; then
        echo "Environment: WSL (Windows Subsystem for Linux)"
    fi
    
    echo ""
    echo "Update command: sudo apt update && sudo apt upgrade"
    echo "Install command: sudo apt install <package>"
    echo "Script command: gscript updateOs"
    
elif is_linux_os; then
    echo "Operating System: Linux (Generic)"
    
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        echo "Distribution: $NAME"
        echo "Version: $VERSION"
    fi
    
    if command -v apt &> /dev/null; then
        echo "Package Manager: apt"
    elif command -v pacman &> /dev/null; then
        echo "Package Manager: pacman"
    elif command -v dnf &> /dev/null; then
        echo "Package Manager: dnf"
    elif command -v yum &> /dev/null; then
        echo "Package Manager: yum"
    elif command -v zypper &> /dev/null; then
        echo "Package Manager: zypper"
    else
        echo "Package Manager: unknown"
    fi
    
else
    echo "Operating System: Unknown"
fi

echo ""
echo "=== Shell Information ==="
echo "Current Shell: $SHELL"
echo "Shell Type: $PROFILE_SHELL"

echo ""
echo "=== Installed Development Tools ==="

check_tool() {
    if command -v "$1" &> /dev/null; then
        version=$($1 --version 2>/dev/null | head -n1 || echo "installed")
        echo "✓ $2: $version"
    else
        echo "✗ $2: not installed"
    fi
}

check_tool "git" "Git"
check_tool "docker" "Docker"
check_tool "go" "Go"
check_tool "rustc" "Rust"
check_tool "node" "Node.js"
check_tool "python3" "Python"
check_tool "java" "Java"
check_tool "dotnet" ".NET"
check_tool "nvim" "Neovim"
check_tool "gh" "GitHub CLI"
check_tool "az" "Azure CLI"
check_tool "terraform" "Terraform"
check_tool "kubectl" "Kubectl"

echo ""
echo "=== Available Commands ==="
echo "myinstall <tool>  - Install development tools"
echo "gscript <script>  - Run general scripts"
echo "gscript help      - List all available scripts"
