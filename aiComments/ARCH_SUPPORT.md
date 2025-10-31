# Arch Linux Support

This project now supports Arch-based distributions including:

-   Arch Linux
-   Manjaro
-   EndeavourOS
-   Garuda Linux
-   Other Arch derivatives

## What's New

### Package Manager Detection

The install scripts now automatically detect and support `pacman` package manager alongside `apt` and `brew`.

### AUR Helper Support

For packages only available in the AUR (Arch User Repository), the scripts will automatically install and use `yay` as an AUR helper if needed.

### Supported Installations

All install scripts now support Arch-based distributions:

-   **Docker** - Installs docker and docker-compose, enables systemd services
-   **GitHub CLI** - Installs gh from official repos
-   **Azure CLI** - Installs from AUR
-   **Terraform** - Installs from official repos
-   **Java** - Installs OpenJDK JRE and JDK
-   **PostgreSQL** - Installs and initializes PostgreSQL with systemd
-   **Neovim** - Installs from official repos
-   **Obsidian** - Installs from AUR
-   **.NET SDK** - Installs dotnet-sdk and aspnet-runtime
-   **Go** - Uses the generic Linux installer (works on all distros)
-   **Rust** - Uses rustup (works on all distros)
-   **Kubectl** - Uses the generic Linux installer (works on all distros)

### System Utilities

The `installStuff.sh` script now installs essential development tools for Arch:

-   base-devel (build tools)
-   git, keychain, gnupg
-   openssl, pkgconf
-   compression tools (zip, unzip, tar, gzip)
-   ripgrep, bat
-   net-tools, fuse2
-   nss (for .NET HTTPS certificates)

### System Updates

New script for Arch system updates:

```bash
gscript updateArch
```

This script:

-   Updates package database and all packages
-   Cleans package cache
-   Identifies and optionally removes orphaned packages
-   Updates AUR packages if yay/paru is installed

## Usage

The scripts work the same way on Arch as on other systems:

```bash
# Install specific tools
myinstall docker
myinstall github
myinstall terraform
myinstall java
myinstall postgres
myinstall neovim
myinstall obsidian
myinstall dotnet

# Install everything
myinstall all

# Install essential dev tools
gscript installStuff

# Update system (Arch)
gscript updateArch

# Update system (Ubuntu/Debian)
gscript updateOs
```

## Technical Details

### New Functions in `commonMyinstallFunctions.sh`

-   `pacman_package_manager_available()` - Checks if pacman is available
-   `is_arch_based()` - Detects Arch-based distributions
-   `ensure_aur_helper_installed()` - Installs yay if needed for AUR packages

### Package Name Mappings

Some packages have different names on Arch:

| Tool        | Debian/Ubuntu                 | Arch                      |
| ----------- | ----------------------------- | ------------------------- |
| Java JRE    | default-jre                   | jre-openjdk               |
| Java JDK    | default-jdk                   | jdk-openjdk               |
| PostgreSQL  | postgresql postgresql-contrib | postgresql                |
| .NET SDK    | dotnet-sdk-8.0                | dotnet-sdk aspnet-runtime |
| Build tools | build-essential               | base-devel                |
| SSL dev     | libssl-dev                    | openssl                   |
| pkg-config  | pkg-config                    | pkgconf                   |
| FUSE        | libfuse2                      | fuse2                     |

### Systemd Services

On Arch, some services need to be explicitly enabled and started:

-   Docker: `docker.service` and `containerd.service`
-   PostgreSQL: `postgresql.service` (also requires database initialization)

## Notes

-   Docker installation on Arch adds the current user to the `docker` group. You may need to log out and back in for this to take effect.
-   PostgreSQL on Arch requires database cluster initialization, which is handled automatically by the install script.
-   AUR packages (Azure CLI, Obsidian) require an AUR helper. The scripts will install `yay` automatically if needed.
