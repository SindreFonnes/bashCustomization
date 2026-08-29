use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

const EZA_APT_KEY_FINGERPRINTS: &[&str] = &["1548BC8A4B4D2688F9B0DAF7EC29E2090CE3FD43"];

#[derive(Debug, Clone, Copy)]
pub struct EzaInstaller;

impl crate::install::Installer for EzaInstaller {
    fn name(&self) -> &str {
        "eza"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("eza")
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_debian() || platform.is_fedora() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform)
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if package_manager::prefers_brew(&config.platform) {
                println!("  Would install eza via brew");
            } else {
                println!("  Would install eza via apt (third-party repo deb.gierens.de)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing eza via brew...");
            return package_manager::brew_install("eza");
        }

        install_eza_apt(platform)
    }
}

fn install_eza_apt(platform: &Platform) -> Result<()> {
    if !platform.is_debian() {
        let distro = platform.distro();
        bail!(
            "third-party repo setup for eza not yet supported on {:?}",
            distro
        );
    }

    println!("Adding eza GPG key...");
    package_manager::apt_add_gpg_key(
        "https://raw.githubusercontent.com/eza-community/eza/main/deb.asc",
        "/etc/apt/keyrings/gierens.gpg",
        EZA_APT_KEY_FINGERPRINTS,
    )?;

    let repo_line =
        "deb [signed-by=/etc/apt/keyrings/gierens.gpg] https://deb.gierens.de stable main";

    println!("Adding eza apt repository...");
    package_manager::apt_add_repo(repo_line, "/etc/apt/sources.list.d/gierens.list")?;

    println!("Installing eza...");
    package_manager::apt_install("eza")?;

    println!("eza installed");
    Ok(())
}
