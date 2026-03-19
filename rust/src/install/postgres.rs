use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

pub struct PostgresInstaller;

impl super::Installer for PostgresInstaller {
    fn name(&self) -> &str {
        "postgres"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("psql")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install postgresql via brew");
            } else {
                println!("  Would install postgresql and postgresql-contrib via apt");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing PostgreSQL via brew...");
            return package_manager::brew_install("postgresql");
        }

        println!("Installing PostgreSQL via apt...");
        package_manager::apt_install("postgresql")?;
        package_manager::apt_install("postgresql-contrib")?;
        println!("PostgreSQL installed");
        Ok(())
    }
}
