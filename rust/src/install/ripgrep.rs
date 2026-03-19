use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct RipgrepInstaller;

impl super::Installer for RipgrepInstaller {
    fn name(&self) -> &str {
        "ripgrep"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("rg")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install ripgrep via brew");
            } else {
                println!("  Would install ripgrep via apt");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing ripgrep via brew...");
            return package_manager::brew_install("ripgrep");
        }

        println!("Installing ripgrep via apt...");
        package_manager::apt_install("ripgrep")
    }
}
