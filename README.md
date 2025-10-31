# Bash Customization & Development Environment

A comprehensive bash/zsh customization and development environment setup with cross-platform support.

## Supported Platforms

-   🍎 **macOS** (Intel & Apple Silicon) - via Homebrew
-   🐧 **Ubuntu/Debian** - via apt
-   🔷 **Arch Linux** and derivatives (Manjaro, EndeavourOS, Garuda) - via pacman
-   🪟 **WSL** (Windows Subsystem for Linux)

## Features

-   ✨ Automatic OS detection and package manager selection
-   📦 One-command installation for 15+ development tools
-   🔄 Unified interface across all platforms
-   🎨 Shell customization (bash/zsh support)
-   🔧 Essential development tools and utilities

## Quick Start

```bash
# Clone the repository
git clone <repo-url> ~/bashCustomization
cd ~/bashCustomization

# Run initialization script
chmod +x ./init_repo.sh && ./init_repo.sh

# Reload your shell
source ~/.bashrc  # or source ~/.zshrc
```

This will:

-   Load shell customizations into your profile
-   Create project directory structure
-   Install essential development tools
-   Configure Git settings

## What Gets Installed

### Essential Tools (via `gscript installStuff`)

-   Build tools (gcc, make, etc.)
-   Git, keychain, gnupg
-   Compression utilities (zip, tar, gzip)
-   Modern CLI tools (ripgrep, bat)
-   Network tools
-   SSL/TLS libraries

### Available Development Tools (via `myinstall`)

-   **Languages**: Go, Rust, Java, .NET, Node.js (via NVM)
-   **Package Managers**: pnpm, yarn, bun
-   **Cloud Tools**: Azure CLI, Terraform, kubectl
-   **Version Control**: GitHub CLI
-   **Databases**: PostgreSQL
-   **Editors**: Neovim
-   **Containers**: Docker + Docker Compose
-   **Apps**: Obsidian

## Usage Examples

```bash
# Check system information
sysinfo

# Install specific tools
myinstall docker
myinstall go
myinstall github

# Install everything
myinstall all

# Update system
gscript updateOs      # Ubuntu/Debian
gscript updateArch    # Arch Linux

# List all available scripts
gscript help
```

## Documentation

-   📖 [Quick Reference Guide](QUICK_REFERENCE.md) - Command cheat sheet
-   🔐 [SSH Configuration](SSH_CONFIG.md) - SSH setup guide
-   ✍️ [GPG Signing Setup](generalScripts/GPG_SIGNING_SETUP.md) - Git commit signing

## Key Commands

| Command            | Description                 |
| ------------------ | --------------------------- |
| `myinstall <tool>` | Install development tools   |
| `gscript <script>` | Run general scripts         |
| `sysinfo`          | Show system information     |
| `gscript help`     | List all available scripts  |
| `updateShell`      | Update shell customizations |

## Project Structure

```
bashCustomization/
├── generalScripts/      # System utilities and setup scripts
├── installScripts/      # Tool installation scripts
│   ├── azure/
│   ├── docker/
│   ├── dotnet/
│   ├── github/
│   ├── go/
│   ├── java/
│   ├── javascript/
│   ├── kubectl/
│   ├── neovim/
│   ├── obsidian/
│   ├── postgres/
│   ├── rust/
│   └── terraform/
├── shellFunctionality/  # Shell enhancements
├── programExtensions/   # Program-specific extensions
├── vim/                # Vim configuration
└── local/              # User-specific customizations
```

## Customization

Add your own customizations to:

```bash
~/bashCustomization/local/local_main.sh
```

The local_aliases and local_variables are sourced automatically and won't be overwritten by updates nor be commited to the repo.

## Platform-Specific Notes

### macOS

-   Automatically installs Homebrew if not present
-   May require Xcode Command Line Tools
-   Uses `brew` for all package management

### Ubuntu/Debian

-   Uses `apt` package manager
-   Some tools add custom repositories
-   Supports WSL environments

### Arch Linux

-   Uses `pacman` for official packages
-   Automatically installs `yay` for AUR packages
-   Manages systemd services (Docker, PostgreSQL)
-   See [ARCH_SUPPORT.md](ARCH_SUPPORT.md) for details

### WSL

-   Automatically detected
-   Special Docker permission handling
-   Windows integration features

## Troubleshooting

### Commands not found after installation

```bash
source ~/.bashrc  # or source ~/.zshrc
```

### Permission issues

```bash
chmod +x ~/bashCustomization/generalScripts/*.sh
```

### Package manager issues

```bash
# Arch
sudo pacman -Sy

# Ubuntu/Debian
sudo apt update

# macOS
brew update
```

For more troubleshooting tips, see [QUICK_REFERENCE.md](QUICK_REFERENCE.md#troubleshooting).
