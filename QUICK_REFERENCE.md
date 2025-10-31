# Quick Reference Guide

## Installation Commands

### Install Individual Tools

```bash
myinstall go          # Install Go
myinstall dotnet      # Install .NET SDK
myinstall rust        # Install Rust
myinstall javascript  # Install Node.js tools (interactive)
myinstall java        # Install Java JDK
myinstall azure       # Install Azure CLI
myinstall github      # Install GitHub CLI
myinstall terraform   # Install Terraform
myinstall brew        # Install Homebrew (macOS/Linux)
myinstall docker      # Install Docker
myinstall nvim        # Install Neovim
myinstall postgres    # Install PostgreSQL
myinstall kubernetes  # Install kubectl
myinstall obsidian    # Install Obsidian
myinstall all         # Install everything
```

### JavaScript Package Managers

```bash
myinstall javascript nvm   # Install NVM
myinstall javascript pnpm  # Install pnpm
myinstall javascript yarn  # Install Yarn
myinstall javascript bun   # Install Bun
```

## General Scripts

### System Information

```bash
gscript detectSystem  # Detect OS and show system info
sysinfo              # Alias for detectSystem
```

### System Updates

```bash
gscript updateOs     # Update Ubuntu/Debian system
gscript updateArch   # Update Arch-based system
```

### Setup Scripts

```bash
gscript installStuff        # Install essential dev tools
gscript installNerdFont     # Install Nerd Fonts
gscript nvimSetup          # Setup Neovim configuration
gscript setupZsh           # Setup Zsh with Oh My Zsh
gscript configureGit       # Configure Git settings
gscript setupGpgSigning    # Setup GPG signing for Git
```

### Utility Scripts

```bash
gscript generateSSLCert                        # Generate SSL certificates
gscript fix_docker_insuficient_permissions_wsl # Fix Docker permissions on WSL
gscript updateDiscord                          # Update Discord
gscript help                                   # List all available scripts
```

## Shell Management

### Reload Configuration

```bash
source ~/.bashrc   # Reload bash configuration
source ~/.zshrc    # Reload zsh configuration
```

### Update Shell Scripts

The shell automatically checks for updates once per day. To force an update:

```bash
updateShell        # Defined in your shell configuration
```

## Platform-Specific Notes

### macOS

-   Uses Homebrew for package management
-   Automatically installs Homebrew if not present
-   Some packages may require Xcode Command Line Tools

### Ubuntu/Debian

-   Uses apt package manager
-   May require sudo password for installations
-   Some packages add custom repositories

### Arch Linux

-   Uses pacman package manager
-   Automatically installs yay for AUR packages
-   Some services require systemd management
-   May need to log out/in after Docker installation

### WSL (Windows Subsystem for Linux)

-   Detected automatically
-   Some features may have limitations
-   Docker requires special permissions setup

## Environment Variables

### Paths

```bash
$bashC              # Bash customization directory
$p_home             # Projects home directory
$notes_home         # Notes directory
$scripts_home       # Scripts directory
```

### System Detection

```bash
$IS_MAC             # true if running on macOS
$IS_WSL             # true if running on WSL
$PROFILE_SHELL      # Current shell (bash/zsh/etc)
```

### Package Managers

```bash
$MYINSTALL_SCRIPT_FOLDER_LOCATION    # Install scripts location
$GENERAL_SCRIPTS_FOLDER_LOCATION     # General scripts location
```

## Common Workflows

### Fresh System Setup

```bash
# 1. Clone the repository
git clone <repo-url> ~/bashCustomization
cd ~/bashCustomization

# 2. Run initialization
chmod +x ./init_repo.sh && ./init_repo.sh

# 3. Reload shell
source ~/.bashrc

# 4. Check system info
sysinfo

# 5. Install tools as needed
myinstall all
```

### Adding New Tools

```bash
# Check what's installed
sysinfo

# Install missing tools
myinstall <tool-name>
```

### Updating System

```bash
# Ubuntu/Debian
gscript updateOs

# Arch Linux
gscript updateArch

# macOS
brew update && brew upgrade
```

## Troubleshooting

### Command Not Found

```bash
# Reload shell configuration
source ~/.bashrc  # or source ~/.zshrc

# Check if scripts are loaded
echo $bashC
```

### Permission Denied

```bash
# Make scripts executable
chmod +x ~/bashCustomization/generalScripts/*.sh
chmod +x ~/bashCustomization/installScripts/**/*.sh
```

### Package Manager Issues

**Arch Linux:**

```bash
# Update package database
sudo pacman -Sy

# Clear package cache
sudo pacman -Sc
```

**Ubuntu/Debian:**

```bash
# Fix broken packages
sudo apt --fix-broken install

# Update package lists
sudo apt update
```

**macOS:**

```bash
# Reinstall Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

### Docker Issues on WSL

```bash
gscript fix_docker_insuficient_permissions_wsl
```

## Tips & Tricks

### Faster Package Installation

On Arch, use parallel downloads:

```bash
# Edit /etc/pacman.conf
ParallelDownloads = 5
```

### Shell Customization

Add custom scripts to:

```bash
~/bashCustomization/local/local_main.sh
```

### Git Configuration

The init script will prompt for Git configuration. To reconfigure:

```bash
gscript configureGit
```

### Neovim Setup

After installing Neovim, run the setup:

```bash
gscript nvimSetup
```

## Getting Help

### List Available Commands

```bash
gscript help        # List all general scripts
myinstall          # Interactive tool selection
sysinfo            # Show system information
```

### Check Installation Status

```bash
sysinfo            # Shows installed tools with versions
```

### Documentation

-   [README.md](README.md) - Main documentation
-   [ARCH_SUPPORT.md](ARCH_SUPPORT.md) - Arch Linux specific info
-   [IMPROVEMENTS.md](IMPROVEMENTS.md) - Recent improvements
-   [SSH_CONFIG.md](SSH_CONFIG.md) - SSH configuration guide
-   [GPG_SIGNING_SETUP.md](generalScripts/GPG_SIGNING_SETUP.md) - GPG setup guide
