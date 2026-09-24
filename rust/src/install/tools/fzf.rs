use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct FzfInstaller;

impl crate::install::Installer for FzfInstaller {
    fn name(&self) -> &str {
        "fzf"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("fzf")
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform) || platform.is_debian() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform)
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if package_manager::prefers_brew(&config.platform) {
                println!("  Would install fzf via brew");
            } else {
                println!("  Would install fzf via package manager");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing fzf via brew...");
            return package_manager::brew_install("fzf");
        }

        println!("Installing fzf via package manager...");
        package_manager::install(&config.platform, "fzf")
    }
}
