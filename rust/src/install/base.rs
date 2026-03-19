use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

pub struct BaseInstaller;

impl super::Installer for BaseInstaller {
    fn name(&self) -> &str {
        "base"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        false // always run to ensure all base packages are present
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install base packages via brew: git, gnupg");
            } else {
                println!("  Would install base packages via apt: build-essential, git, safe-rm, keychain, nala, gnupg, etc.");
            }
            return Ok(());
        }

        if platform.is_mac() {
            install_base_mac()?;
        } else if platform.is_linux() {
            install_base_linux()?;
        }

        Ok(())
    }

    fn phase(&self) -> u8 {
        0 // base phase
    }
}

fn install_base_mac() -> Result<()> {
    println!("Installing base packages via brew...");
    let packages = ["git", "gnupg"];
    for pkg in &packages {
        if let Err(e) = package_manager::brew_install(pkg) {
            println!("  Warning: failed to install {pkg}: {e}");
        }
    }
    Ok(())
}

fn install_base_linux() -> Result<()> {
    println!("Adding universe repository...");
    if command::is_root() {
        let _ = command::run_visible("add-apt-repository", &["universe", "-y"]);
    } else {
        let _ = command::run_sudo("add-apt-repository", &["universe", "-y"]);
    }

    let packages = [
        "build-essential",
        "git",
        "safe-rm",
        "keychain",
        "nala",
        "gnupg",
        "pkg-config",
        "libssl-dev",
        "zip",
        "unzip",
        "tar",
        "gzip",
        "net-tools",
        "libfuse2",
        "libnss3-tools",
    ];

    println!("Installing base packages via apt...");

    if command::is_root() {
        command::run_visible("apt-get", &["update"])?;
    } else {
        command::run_sudo("apt-get", &["update"])?;
    }

    let mut args = vec!["install", "-y"];
    let pkg_refs: Vec<&str> = packages.iter().copied().collect();
    args.extend_from_slice(&pkg_refs);

    if command::is_root() {
        command::run_visible("apt-get", &args)?;
    } else {
        command::run_sudo("apt-get", &args)?;
    }

    println!("Base packages installed");
    Ok(())
}
