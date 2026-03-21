use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct ShellcheckInstaller;

impl crate::install::Installer for ShellcheckInstaller {
    fn name(&self) -> &str {
        "shellcheck"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("shellcheck")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install shellcheck via brew");
            } else {
                println!("  Would install shellcheck via package manager");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing shellcheck via brew...");
            return package_manager::brew_install("shellcheck");
        }

        println!("Installing shellcheck via package manager...");
        package_manager::install(&config.platform, "shellcheck")
    }
}
