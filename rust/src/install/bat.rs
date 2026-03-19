use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct BatInstaller;

impl super::Installer for BatInstaller {
    fn name(&self) -> &str {
        "bat"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("bat") || command::exists("batcat")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install bat via brew");
            } else {
                println!("  Would install bat via apt (with batcat -> bat symlink)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing bat via brew...");
            return package_manager::brew_install("bat");
        }

        println!("Installing bat via apt...");
        package_manager::apt_install("bat")?;

        // On Debian/Ubuntu, bat is installed as batcat — create symlink
        if command::exists("batcat") && !command::exists("bat") {
            let local_bin = format!("{}/.local/bin", std::env::var("HOME").unwrap_or_default());
            std::fs::create_dir_all(&local_bin)?;
            let symlink_path = format!("{local_bin}/bat");
            if !std::path::Path::new(&symlink_path).exists() {
                let batcat_path = command::run("which", &["batcat"])?;
                std::os::unix::fs::symlink(batcat_path.trim(), &symlink_path)?;
                println!("Created symlink {symlink_path} -> batcat");
            }
        }

        Ok(())
    }
}
