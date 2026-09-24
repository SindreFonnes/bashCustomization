use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct LazygitInstaller;

impl crate::install::Installer for LazygitInstaller {
    fn name(&self) -> &str {
        "lazygit"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        command::exists("lazygit")
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform) || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform)
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!("  Would install lazygit via brew");
            return Ok(());
        }

        // Older Debian/Ubuntu releases do not package lazygit. Use Homebrew
        // consistently rather than assuming an apt package is available.
        if package_manager::is_brew_failed() || !package_manager::has_brew() {
            bail!("Homebrew is required to install lazygit; retry with bashc install brew");
        }

        println!("Installing lazygit via brew...");
        package_manager::brew_install("lazygit")
    }
}
