use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, bail};

use super::command;
use super::platform::Platform;
use super::privilege;

static BREW_FAILED: AtomicBool = AtomicBool::new(false);

/// Mark brew as failed for the remainder of this run.
pub fn set_brew_failed() {
    BREW_FAILED.store(true, Ordering::SeqCst);
}

/// Check if brew installation failed earlier in this run.
pub fn is_brew_failed() -> bool {
    BREW_FAILED.load(Ordering::SeqCst)
}

/// Check if brew is available on PATH.
pub fn has_brew() -> bool {
    command::exists("brew")
}

/// Ensure Homebrew is installed. On macOS: /opt/homebrew or /usr/local.
/// On Debian/Fedora Linux: Linuxbrew at /home/linuxbrew/.linuxbrew.
pub fn ensure_brew(platform: &Platform) -> Result<()> {
    if has_brew() {
        return Ok(());
    }

    if is_brew_failed() {
        bail!("Homebrew installation previously failed this run — skipping");
    }

    println!("Installing Homebrew...");

    if platform.is_mac() {
        // Official Homebrew installer
        let result = command::run_visible(
            "bash",
            &[
                "-c",
                "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
            ],
        );

        if result.is_err() {
            set_brew_failed();
            return result;
        }

        // Activate brew in current session
        if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
            let shellenv = command::run("/opt/homebrew/bin/brew", &["shellenv"])?;
            command::run_visible("bash", &["-c", &shellenv])?;
        }
    } else if platform.is_linux() {
        let result = command::run_visible(
            "bash",
            &[
                "-c",
                "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
            ],
        );

        if result.is_err() {
            set_brew_failed();
            return result;
        }

        // Activate Linuxbrew in current session
        if std::path::Path::new("/home/linuxbrew/.linuxbrew/bin/brew").exists() {
            let shellenv =
                command::run("/home/linuxbrew/.linuxbrew/bin/brew", &["shellenv"])?;
            command::run_visible("bash", &["-c", &shellenv])?;
        }
    }

    if !has_brew() {
        set_brew_failed();
        bail!("Homebrew installation completed but brew is not on PATH");
    }

    Ok(())
}

/// Install a package via brew.
pub fn brew_install(package: &str) -> Result<()> {
    command::run_visible("brew", &["install", package])
}

/// Install a brew cask (macOS only).
pub fn brew_install_cask(package: &str) -> Result<()> {
    command::run_visible("brew", &["install", "--cask", package])
}

/// Install a package using the preferred method for the platform.
/// Tries brew first on macOS/Debian/Fedora, falls back to apt.
pub fn install(platform: &Platform, package: &str) -> Result<()> {
    if !is_brew_failed() && has_brew() {
        return brew_install(package);
    }

    if platform.is_linux() {
        return apt_install(package);
    }

    bail!("No package manager available to install {package}")
}

/// Install a package via apt.
pub fn apt_install(package: &str) -> Result<()> {
    privilege::run_privileged("apt-get", &["install", "-y", package])
}

/// Download a GPG key and install it for apt.
pub fn apt_add_gpg_key(url: &str, keyring_path: &str) -> Result<()> {
    // Download key bytes and pipe to gpg --dearmor
    let key_data = command::run(
        "curl",
        &["-fsSL", url],
    )?;

    // Ensure /etc/apt/keyrings exists
    privilege::run_privileged("mkdir", &["-p", "/etc/apt/keyrings"])?;

    // Pipe key through gpg --dearmor to keyring_path
    let cmd = format!(
        "echo '{}' | gpg --dearmor -o {}",
        key_data.replace('\'', "'\\''"),
        keyring_path
    );

    privilege::run_privileged("bash", &["-c", &cmd])
}

/// Add an apt repository source file and run apt update.
pub fn apt_add_repo(repo_line: &str, list_file: &str) -> Result<()> {
    let cmd = format!("echo '{}' | tee {}", repo_line, list_file);

    privilege::run_privileged("bash", &["-c", &cmd])?;
    privilege::run_privileged("apt-get", &["update"])
}

/// Returns true if on Linux and not root (needs sudo for apt).
pub fn needs_sudo_for_apt(platform: &Platform) -> bool {
    platform.is_linux() && !command::is_root()
}
