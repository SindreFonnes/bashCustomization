use anyhow::Result;

use crate::common::{package_manager, platform::Platform, privilege};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct BaseInstaller;

impl crate::install::Installer for BaseInstaller {
    fn name(&self) -> &str {
        "base"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // Base packages on Debian always use apt, which needs root
        platform.is_debian()
    }

    fn is_installed(&self) -> bool {
        false // always run to ensure all base packages are present
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install base packages via brew: git, gnupg");
            } else if platform.is_debian() {
                println!("  Would install base packages via apt: build-essential, git, safe-rm, keychain, nala, gnupg, etc.");
            } else if let Some(distro) = platform.distro() {
                println!("  base packages not yet configured for {distro:?}");
            }
            return Ok(());
        }

        if platform.is_mac() {
            install_base_mac()?;
        } else if platform.is_debian() {
            install_base_linux()?;
        } else if platform.is_nixos() {
            return package_manager::nix_guidance("base development tools");
        } else if platform.is_linux() {
            if let Some(distro) = platform.distro() {
                anyhow::bail!("base packages not yet configured for {distro:?}");
            } else {
                anyhow::bail!("base packages not supported on this platform");
            }
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
    let _ = privilege::run_privileged("add-apt-repository", &["universe", "-y"]);

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

    privilege::run_privileged("apt-get", &["update"])?;

    let mut args = vec!["install", "-y"];
    let pkg_refs: Vec<&str> = packages.iter().copied().collect();
    args.extend_from_slice(&pkg_refs);

    privilege::run_privileged("apt-get", &args)?;

    println!("Base packages installed");
    Ok(())
}
