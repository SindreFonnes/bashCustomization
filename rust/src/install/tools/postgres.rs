use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::{InstallConfig, InstallationState};

#[derive(Debug, Clone, Copy)]
pub struct PostgresInstaller;

impl crate::install::Installer for PostgresInstaller {
    fn name(&self) -> &str {
        "postgres"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("psql")
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform) || platform.is_debian() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform)
    }

    fn installation_state(&self, platform: &Platform) -> InstallationState {
        if !command::exists("psql") {
            return InstallationState::Missing;
        }

        if platform.is_debian()
            && !package_manager::has_brew()
            && !debian_package_installed("postgresql-contrib")
        {
            InstallationState::Incomplete("postgresql-contrib package is missing".to_string())
        } else {
            InstallationState::Complete
        }
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if package_manager::prefers_brew(&config.platform) {
                println!("  Would install postgresql via brew");
            } else {
                println!("  Would install postgresql and postgresql-contrib via package manager");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing PostgreSQL via brew...");
            return package_manager::brew_install("postgresql");
        }

        println!("Installing PostgreSQL via package manager...");
        package_manager::install(&config.platform, "postgresql")?;
        package_manager::install(&config.platform, "postgresql-contrib")?;
        println!("PostgreSQL installed");
        Ok(())
    }
}

fn debian_package_installed(package: &str) -> bool {
    command::run("dpkg-query", &["-W", "-f=${Status}", package])
        .map(|status| status.contains("install ok installed"))
        .unwrap_or(false)
}
