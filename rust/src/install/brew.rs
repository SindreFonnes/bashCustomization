use anyhow::Result;

use crate::common::{package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct BrewInstaller;

impl super::Installer for BrewInstaller {
    fn name(&self) -> &str {
        "brew"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        package_manager::has_brew()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!("  Would install Homebrew");
            return Ok(());
        }

        println!("Installing Homebrew...");
        package_manager::ensure_brew(&config.platform)
    }

    fn phase(&self) -> u8 {
        0 // base phase — must run before other installers
    }
}
