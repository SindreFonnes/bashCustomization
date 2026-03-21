use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct FdInstaller;

impl crate::install::Installer for FdInstaller {
    fn name(&self) -> &str {
        "fd"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("fd") || command::exists("fdfind")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install fd via brew");
            } else if config.platform.is_debian() {
                println!("  Would install fd-find via apt (with fdfind -> fd symlink)");
            } else {
                println!("  Would install fd via package manager");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing fd via brew...");
            return package_manager::brew_install("fd");
        }

        println!("Installing fd via package manager...");

        // Debian/Ubuntu packages fd as fd-find; other distros use fd or fd-find
        let package = if config.platform.is_debian() {
            "fd-find"
        } else {
            "fd"
        };
        package_manager::install(&config.platform, package)?;

        // On Debian/Ubuntu, fd is installed as fdfind — create symlink
        if config.platform.is_debian() {
            if command::exists("fdfind") && !command::exists("fd") {
                let local_bin =
                    format!("{}/.local/bin", std::env::var("HOME").unwrap_or_default());
                std::fs::create_dir_all(&local_bin)?;
                let symlink_path = format!("{local_bin}/fd");
                if !std::path::Path::new(&symlink_path).exists() {
                    let fdfind_path = command::run("which", &["fdfind"])?;
                    std::os::unix::fs::symlink(fdfind_path.trim(), &symlink_path)?;
                    println!("Created symlink {symlink_path} -> fdfind");
                }
            }
        }

        Ok(())
    }
}
