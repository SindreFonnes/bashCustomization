use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct JavaInstaller;

impl crate::install::Installer for JavaInstaller {
    fn name(&self) -> &str {
        "java"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("java")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install openjdk via brew");
            } else {
                println!("  Would install default-jre and default-jdk via package manager");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Java (OpenJDK) via brew...");
            return package_manager::brew_install("openjdk");
        }

        println!("Installing Java via package manager...");
        package_manager::install(&config.platform, "default-jre")?;
        package_manager::install(&config.platform, "default-jdk")?;
        println!("Java installed");
        Ok(())
    }
}
