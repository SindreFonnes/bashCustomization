#!/usr/bin/env bash

set -euo pipefail

# Source common install functions if running directly
if [[ -n "${MYINSTALL_COMMON_FUNCTIONS_LOCATION:-}" ]]; then
    source "$MYINSTALL_COMMON_FUNCTIONS_LOCATION"
elif [[ -n "${bashC:-}" ]]; then
    source "$bashC/installScripts/commonMyinstallFunctions.sh"
else
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    source "$SCRIPT_DIR/../installScripts/commonMyinstallFunctions.sh"
fi

# Install dependencies for macOS
install_macos_dependencies() {
    echo "Detected macOS..." >&2
    
    # Ensure brew is installed and get its prefix
    local BREW_PREFIX
    if ! BREW_PREFIX=$(ensure_brew_installed); then
        echo "ERROR: Failed to install or locate Homebrew" >&2
        exit 1
    fi
    
    echo "Installing dependencies (gnupg, pinentry-mac, gh)..." >&2
    brew install gnupg pinentry-mac gh >/dev/null
    
    local GPG_BIN="$(command -v gpg)"
    local PINENTRY_BIN="${BREW_PREFIX}/bin/pinentry-mac"
    
    # Configure pinentry
    echo "Configuring pinentry..." >&2
    local GPG_AGENT_CONF=~/.gnupg/gpg-agent.conf
    if ! grep -q "^pinentry-program " "$GPG_AGENT_CONF" 2>/dev/null; then
        echo "pinentry-program ${PINENTRY_BIN}" >> "$GPG_AGENT_CONF"
    else
        sed -i.bak "s|^pinentry-program .*|pinentry-program ${PINENTRY_BIN}|" "$GPG_AGENT_CONF"
    fi
    
    echo "$GPG_BIN"
}

# Install dependencies for Ubuntu/Debian
install_ubuntu_dependencies() {
    echo "Detected Ubuntu/Debian-based system..." >&2
    
    # Check if apt is available
    if ! apt_package_manager_available; then
        echo "ERROR: apt-get not found. This script only supports Ubuntu/Debian-based distributions." >&2
        echo "For other distributions (Arch, Fedora, etc.), please install GPG and GitHub CLI manually." >&2
        exit 1
    fi
    
    if ! command -v gpg >/dev/null; then
        echo "Installing GnuPG and pinentry..." >&2
        sudo apt-get update -y
        # Choose a pinentry based on environment
        if command -v gsettings >/dev/null || [[ -n "${XDG_CURRENT_DESKTOP:-}" ]]; then
            sudo apt-get install -y gnupg pinentry-gnome3
        else
            sudo apt-get install -y gnupg pinentry-curses
        fi
    fi
    
    if ! command -v gh >/dev/null; then
        echo "Installing GitHub CLI (gh)..." >&2
        type -p curl >/dev/null || sudo apt-get install -y curl
        curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | \
            sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
        sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | \
            sudo tee /etc/apt/sources.list.d/github-cli.list >/dev/null
        sudo apt-get update -y && sudo apt-get install -y gh
    fi
    
    local GPG_BIN="$(command -v gpg)"
    echo "$GPG_BIN"
}

# Main setup logic
main() {
    local GPG_BIN
    
    # Use common functions to detect OS and install dependencies
    if is_mac_os; then
        GPG_BIN=$(install_macos_dependencies)
    elif is_ubuntu_debian || apt_package_manager_available; then
        GPG_BIN=$(install_ubuntu_dependencies)
    else
        echo "ERROR: Unsupported operating system." >&2
        echo "This script only supports macOS and Ubuntu/Debian-based systems." >&2
        echo "For Arch, Fedora, or other distributions, please manually install:" >&2
        echo "  - gnupg" >&2
        echo "  - pinentry (appropriate for your system)" >&2
        echo "  - gh (GitHub CLI)" >&2
        exit 1
    fi
    
    # Ensure ~/.gnupg exists with correct permissions
    mkdir -p ~/.gnupg
    chmod 700 ~/.gnupg
    
    # Restart gpg-agent to pick up changes
    gpgconf --kill gpg-agent || true
    
    # Determine shell profile file
    local PROFILE_FILE="${HOME}/.bashrc"
    if [[ -n "${SHELL:-}" ]]; then
        if [[ "${SHELL}" == *zsh* ]]; then
            PROFILE_FILE="${HOME}/.zshrc"
        fi
    fi
    
    # Ensure GPG_TTY is exported in shell profile
    if ! grep -q "export GPG_TTY" "${PROFILE_FILE}" 2>/dev/null; then
        echo 'export GPG_TTY=$(tty)' >> "${PROFILE_FILE}"
        echo "Added GPG_TTY to ${PROFILE_FILE}"
    fi
    
    echo
    echo "=== GPG Key Generation ==="
    echo
    
    # Gather user information
    read -rp "Full name (as in Git commits): " NAME
    read -rp "Email (MUST be verified on GitHub): " EMAIL
    read -rp "Key comment (optional, e.g. 'Git signing'): " COMMENT
    read -rp "Key expiration (e.g. 2y, 1y, 0 = never) [default: 2y]: " EXPIRE
    EXPIRE=${EXPIRE:-2y}
    
    local GPG_UID="$NAME <$EMAIL>"
    local LABEL="${COMMENT:-Git signing key}"
    
    echo
    echo "Generating Ed25519 signing key for: ${GPG_UID} (expires: ${EXPIRE})"
    
    # Generate the GPG key
    ${GPG_BIN} --quick-generate-key "${GPG_UID}" ed25519 sign "${EXPIRE}"
    
    # Retrieve the key fingerprint
    local FPR="$(${GPG_BIN} --list-secret-keys --with-colons "${EMAIL}" | awk -F: '/^fpr:/ {print $10; exit}')"
    if [[ -z "${FPR}" ]]; then
        echo "ERROR: Could not locate generated key fingerprint." >&2
        exit 1
    fi
    
    echo "Key fingerprint: ${FPR}"
    
    # Configure Git to use the GPG key
    git config --global gpg.program "${GPG_BIN}"
    git config --global user.signingkey "${FPR}"
    git config --global commit.gpgsign true
    git config --global tag.gpgsign true
    
    echo "✅ Git configured to use GPG signing"
    
    # Export the public key
    local PUBFILE="${HOME}/.gnupg/${FPR}.asc"
    ${GPG_BIN} --armor --export "${FPR}" > "${PUBFILE}"
    
    echo
    echo "Your public key was saved to: ${PUBFILE}"
    echo "A preview follows:"
    echo "---------------------------------"
    head -n 20 "${PUBFILE}"
    echo "..."
    tail -n 5 "${PUBFILE}"
    echo "---------------------------------"
    
    # Offer to add key to GitHub
    if command -v gh >/dev/null; then
        echo
        read -rp "Add this GPG key to your GitHub account now? (y/N): " ADDGH
        if [[ "${ADDGH:-N}" =~ ^[Yy]$ ]]; then
            echo "Adding key to GitHub (you may be prompted to authenticate)..."
            gh auth status >/dev/null 2>&1 || gh auth login
            if gh gpg-key add "${PUBFILE}" --title "${LABEL}"; then
                echo "✅ Key added to GitHub."
            else
                echo "⚠️  Failed to add key to GitHub. You can add it manually in Settings → SSH and GPG keys."
            fi
        else
            echo "You can add it later in GitHub Settings → SSH and GPG keys."
        fi
    fi
    
    echo
    echo "=== Setup Complete! ==="
    echo
    echo "⚠️  Important: Open a new shell or run 'source ${PROFILE_FILE}' for GPG_TTY to take effect."
    echo
    echo "Test your setup with:"
    echo "  git commit --allow-empty -m 'test signed commit'"
    echo "  git push"
    echo
    echo "The commit should show as 'Verified' on GitHub."
}

# Run main function
main
