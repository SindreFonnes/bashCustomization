#!/usr/bin/env bash

set -euo pipefail

# Source common install functions if running directly
if [[ -n "${MYINSTALL_COMMON_FUNCTIONS_LOCATION:-}" ]]; then
    # shellcheck source=installScripts/commonMyinstallFunctions.sh
    source "$MYINSTALL_COMMON_FUNCTIONS_LOCATION"
elif [[ -n "${bashC:-}" ]]; then
    # shellcheck source=installScripts/commonMyinstallFunctions.sh
    source "$bashC/installScripts/commonMyinstallFunctions.sh"
else
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    # shellcheck source=installScripts/commonMyinstallFunctions.sh
    source "$SCRIPT_DIR/../installScripts/commonMyinstallFunctions.sh"
fi

# Check executables before choosing a package manager so existing installations
# (including tools installed by another manager) are reused.
install_gpg_dependencies() {
    local package_manager=""
    local pinentry_command=pinentry
    local gnupg_package=gnupg
    local pinentry_package=pinentry
    local -a missing_packages=()

    if [[ "$OSTYPE" == darwin* ]]; then
        pinentry_command=pinentry-mac
    fi

    if command -v gpg >/dev/null 2>&1 &&
       command -v gpgconf >/dev/null 2>&1 &&
       command -v "$pinentry_command" >/dev/null 2>&1 &&
       command -v gh >/dev/null 2>&1; then
        command -v gpg
        return
    fi

    if command -v brew >/dev/null 2>&1; then
        package_manager=brew
    elif [[ "$OSTYPE" == darwin* ]]; then
        # Only bootstrap Homebrew when a dependency is actually missing.
        ensure_brew_installed >/dev/null || return $?
        package_manager=brew
    elif command -v dnf >/dev/null 2>&1; then
        package_manager=dnf
        gnupg_package=gnupg2
    elif command -v apt-get >/dev/null 2>&1; then
        package_manager=apt-get
        pinentry_package=pinentry-curses
    else
        echo "ERROR: Missing GPG setup dependencies and no supported package manager (brew, dnf, apt-get)." >&2
        return 1
    fi

    if [[ "$OSTYPE" == darwin* ]]; then
        pinentry_package=pinentry-mac
    fi

    if ! command -v gpg >/dev/null 2>&1 || ! command -v gpgconf >/dev/null 2>&1; then
        missing_packages+=("$gnupg_package")
    fi
    if ! command -v "$pinentry_command" >/dev/null 2>&1; then
        missing_packages+=("$pinentry_package")
    fi
    if ! command -v gh >/dev/null 2>&1; then
        missing_packages+=(gh)
    fi

    # stdout is captured as GPG_BIN; installer output must go to stderr.
    echo "Installing missing dependencies via $package_manager: ${missing_packages[*]}" >&2
    case "$package_manager" in
        brew) brew install "${missing_packages[@]}" >&2 || return $? ;;
        dnf) sudo dnf install -y "${missing_packages[@]}" >&2 || return $? ;;
        apt-get)
            sudo apt-get update >&2 || return $?
            sudo apt-get install -y "${missing_packages[@]}" >&2 || return $?
            ;;
    esac
    command -v gpg
}

# Main setup logic
main() {
    local GPG_BIN
    GPG_BIN=$(install_gpg_dependencies) || return $?

    # Ensure ~/.gnupg exists with correct permissions
    mkdir -p ~/.gnupg
    chmod 700 ~/.gnupg

    if [[ "$OSTYPE" == darwin* ]]; then
        local pinentry_bin
        local gpg_agent_conf="$HOME/.gnupg/gpg-agent.conf"
        pinentry_bin=$(command -v pinentry-mac) || return $?
        if ! grep -q "^pinentry-program " "$gpg_agent_conf" 2>/dev/null; then
            printf 'pinentry-program %s\n' "$pinentry_bin" >> "$gpg_agent_conf"
        else
            sed -i.bak "s|^pinentry-program .*|pinentry-program ${pinentry_bin}|" "$gpg_agent_conf"
        fi
    fi
    
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
        # Expanded by the user's shell when it reads the profile.
        # shellcheck disable=SC2016
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
    local FPR
    FPR="$(${GPG_BIN} --list-secret-keys --with-colons "${EMAIL}" | awk -F: '/^fpr:/ {print $10; exit}')"
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
