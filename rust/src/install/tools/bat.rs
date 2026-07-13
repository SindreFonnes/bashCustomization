use anyhow::Result;

use crate::common::{self, command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct BatInstaller;

impl crate::install::Installer for BatInstaller {
    fn name(&self) -> &str {
        "bat"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("bat") || command::exists("batcat")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install bat via brew");
            } else if config.platform.is_debian() {
                println!("  Would install bat via apt (with batcat -> bat symlink)");
            } else {
                println!("  Would install bat via package manager");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing bat via brew...");
            return package_manager::brew_install("bat");
        }

        println!("Installing bat via package manager...");
        package_manager::install(&config.platform, "bat")?;

        // On Debian/Ubuntu, bat is installed as batcat — create symlink
        if config.platform.is_debian() && command::exists("batcat") && !command::exists("bat") {
            let local_bin = common::home_dir()?.join(".local").join("bin");
            std::fs::create_dir_all(&local_bin)?;
            let symlink_path = local_bin.join("bat");
            if !symlink_path.exists() {
                let batcat_path = command::run("which", &["batcat"])?;
                std::os::unix::fs::symlink(batcat_path.trim(), &symlink_path)?;
                println!("Created symlink {} -> batcat", symlink_path.display());
            }
        }

        Ok(())
    }
}
