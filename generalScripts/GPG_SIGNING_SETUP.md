# GPG Signing Setup Script

## Overview

The `setupGpgSigning.sh` script provides an automated way to set up GPG signing for Git commits on both macOS and Ubuntu/Debian-based systems. It combines the functionality of separate macOS and Ubuntu scripts into a single, intelligent script that detects your operating system and configures everything accordingly.

## Usage

Run the script using the general script runner:

```bash
gscript setupGpgSigning
```

Or run it directly:

```bash
/Users/sindre.fonnes/bashCustomization/generalScripts/setupGpgSigning.sh
```

## What It Does

1. **Detects Your Operating System**

    - Automatically identifies macOS, Ubuntu/Debian, or unsupported systems
    - Provides clear error messages for unsupported distributions (Arch, Fedora, etc.)

2. **Installs Required Dependencies**

    - **macOS**: Installs via Homebrew
        - `gnupg` - GPG encryption software
        - `pinentry-mac` - Secure PIN entry for macOS
        - `gh` - GitHub CLI
    - **Ubuntu/Debian**: Installs via apt
        - `gnupg` - GPG encryption software
        - `pinentry-gnome3` or `pinentry-curses` (based on desktop environment)
        - `gh` - GitHub CLI

3. **Configures GPG**

    - Creates `~/.gnupg` directory with correct permissions (700)
    - Configures appropriate pinentry program for your system
    - Sets up `GPG_TTY` environment variable in your shell profile

4. **Generates GPG Key**

    - Creates an Ed25519 key (modern, secure, fast)
    - Prompts for:
        - Full name (as it appears in Git commits)
        - Email (must be verified on GitHub)
        - Optional comment
        - Expiration period (default: 2 years)

5. **Configures Git**

    - Sets `gpg.program` to the correct GPG binary
    - Sets `user.signingkey` to your new key fingerprint
    - Enables automatic signing for commits (`commit.gpgsign = true`)
    - Enables automatic signing for tags (`tag.gpgsign = true`)

6. **GitHub Integration**
    - Exports your public key to `~/.gnupg/{fingerprint}.asc`
    - Offers to automatically add the key to your GitHub account via GitHub CLI
    - Handles authentication if needed

## Supported Operating Systems

### ✅ Fully Supported

-   macOS (Apple Silicon and Intel)
-   Ubuntu
-   Debian-based distributions

### ❌ Not Supported

-   Arch Linux
-   Fedora
-   Other distributions without `apt-get`

For unsupported systems, the script will provide clear instructions on what to install manually.

## Example Run

```
$ gscript setupGpgSigning

Detected macOS...
Installing dependencies (gnupg, pinentry-mac, gh)...
Configuring pinentry...

=== GPG Key Generation ===

Full name (as in Git commits): John Doe
Email (MUST be verified on GitHub): john.doe@example.com
Key comment (optional, e.g. 'Git signing'): Work laptop
Key expiration (e.g. 2y, 1y, 0 = never) [default: 2y]: 2y

Generating Ed25519 signing key for: John Doe <john.doe@example.com> (expires: 2y)
Key fingerprint: 1234567890ABCDEF1234567890ABCDEF12345678
✅ Git configured to use GPG signing

Your public key was saved to: /Users/john/.gnupg/1234567890ABCDEF1234567890ABCDEF12345678.asc
A preview follows:
---------------------------------
-----BEGIN PGP PUBLIC KEY BLOCK-----
...
-----END PGP PUBLIC KEY BLOCK-----
---------------------------------

Add this GPG key to your GitHub account now? (y/N): y
Adding key to GitHub (you may be prompted to authenticate)...
✅ Key added to GitHub.

=== Setup Complete! ===

⚠️  Important: Open a new shell or run 'source ~/.zshrc' for GPG_TTY to take effect.

Test your setup with:
  git commit --allow-empty -m 'test signed commit'
  git push

The commit should show as 'Verified' on GitHub.
```

## Testing Your Setup

After running the script:

1. **Start a new shell** or reload your profile:

    ```bash
    source ~/.zshrc  # or ~/.bashrc
    ```

2. **Make a test commit**:

    ```bash
    git commit --allow-empty -m 'test signed commit'
    git push
    ```

3. **Check on GitHub**: The commit should show a "Verified" badge

## Troubleshooting

### "failed to sign the data" error

If you get this error when committing:

```
error: gpg failed to sign the data
fatal: failed to write commit object
```

**Solution**: Make sure you've reloaded your shell profile to get the `GPG_TTY` variable:

```bash
source ~/.zshrc  # or ~/.bashrc
```

### Pinentry doesn't appear

**macOS**: Make sure you installed `pinentry-mac` and restarted `gpg-agent`:

```bash
brew install pinentry-mac
gpgconf --kill gpg-agent
```

**Ubuntu**: Make sure the appropriate pinentry is installed:

```bash
sudo apt-get install pinentry-gnome3  # or pinentry-curses
```

### Key not showing as verified on GitHub

1. Make sure your email in the GPG key matches an email verified on GitHub
2. Make sure you added the public key to GitHub (Settings → SSH and GPG keys)
3. Check that commit signing is enabled:
    ```bash
    git config --global commit.gpgsign
    # Should output: true
    ```

## Technical Details

-   **Key Type**: Ed25519 (modern elliptic curve cryptography)
-   **Key Capability**: Signing only (not encryption)
-   **Default Expiration**: 2 years (recommended for security)
-   **Security**: Keys are stored in `~/.gnupg/` with 700 permissions

## Script Architecture

The script uses a modular design that leverages the project's common utility functions:

1. **Common Functions** (from `commonMyinstallFunctions.sh`)

    - `is_mac_os()` - Detects macOS and ensures Homebrew is installed
    - `is_ubuntu_debian()` - Detects Ubuntu/Debian-based systems
    - `apt_package_manager_available()` - Checks for apt package manager
    - `ensure_brew_installed()` - Installs Homebrew and returns its prefix path

2. **Script-Specific Functions**
    - `install_macos_dependencies()` - Handles macOS-specific setup
    - `install_ubuntu_dependencies()` - Handles Ubuntu/Debian-specific setup
    - `main()` - Orchestrates the entire setup process

This design:

-   Maintains consistency with other install scripts in the project
-   Reuses battle-tested utility functions
-   Makes it easy to add support for additional operating systems in the future
-   Ensures proper Homebrew installation (without the dangerous `sudo` bug that was fixed)
