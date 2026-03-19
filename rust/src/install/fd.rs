use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

pub struct FdInstaller;

impl super::Installer for FdInstaller {
    fn name(&self) -> &str {
        "fd"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("fd") || command::exists("fdfind")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install fd via brew");
            } else {
                println!("  Would install fd-find via apt (with fdfind -> fd symlink)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing fd via brew...");
            return package_manager::brew_install("fd");
        }

        println!("Installing fd via apt...");
        package_manager::apt_install("fd-find")?;

        // On Debian/Ubuntu, fd is installed as fdfind — create symlink
        if command::exists("fdfind") && !command::exists("fd") {
            let local_bin = format!("{}/.local/bin", std::env::var("HOME").unwrap_or_default());
            std::fs::create_dir_all(&local_bin)?;
            let symlink_path = format!("{local_bin}/fd");
            if !std::path::Path::new(&symlink_path).exists() {
                let fdfind_path = command::run("which", &["fdfind"])?;
                std::os::unix::fs::symlink(fdfind_path.trim(), &symlink_path)?;
                println!("Created symlink {symlink_path} -> fdfind");
            }
        }

        Ok(())
    }
}
