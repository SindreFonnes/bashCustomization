use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct EzaInstaller;

impl crate::install::Installer for EzaInstaller {
    fn name(&self) -> &str {
        "eza"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("eza")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
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

        install_eza_apt()
    }
}

fn install_eza_apt() -> Result<()> {
    println!("Adding eza GPG key...");
    package_manager::apt_add_gpg_key(
        "https://raw.githubusercontent.com/eza-community/eza/main/deb.asc",
        "/etc/apt/keyrings/gierens.gpg",
    )?;

    let repo_line =
        "deb [signed-by=/etc/apt/keyrings/gierens.gpg] http://deb.gierens.de stable main";

    println!("Adding eza apt repository...");
    package_manager::apt_add_repo(repo_line, "/etc/apt/sources.list.d/gierens.list")?;

    println!("Installing eza...");
    package_manager::apt_install("eza")?;

    println!("eza installed");
    Ok(())
}
