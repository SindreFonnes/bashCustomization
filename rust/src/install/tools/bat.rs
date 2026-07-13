use anyhow::{Context, Result, bail};

use crate::common::{self, command, package_manager, platform::Platform};
use crate::install::{InstallConfig, InstallationState};

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
        canonical_bat_exists()
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform) || platform.is_debian() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_fedora()
    }

    fn installation_state(&self, platform: &Platform) -> InstallationState {
        classify_bat_state(
            canonical_bat_exists(),
            command::exists("batcat"),
            platform.is_debian(),
        )
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
        if config.platform.is_debian() {
            ensure_bat_compatibility()?;
        }

        Ok(())
    }
}

fn classify_bat_state(
    canonical_exists: bool,
    fallback_exists: bool,
    is_debian: bool,
) -> InstallationState {
    if canonical_exists {
        InstallationState::Complete
    } else if is_debian && fallback_exists {
        InstallationState::Incomplete(
            "batcat exists but the bat compatibility command is missing".to_string(),
        )
    } else {
        InstallationState::Missing
    }
}

fn bat_compatibility_path() -> Option<std::path::PathBuf> {
    common::home_dir()
        .ok()
        .map(|home| home.join(".local").join("bin").join("bat"))
}

fn canonical_bat_exists() -> bool {
    command::exists("bat")
        || bat_compatibility_path()
            .map(|path| path.exists())
            .unwrap_or(false)
}

fn ensure_bat_compatibility() -> Result<()> {
    if canonical_bat_exists() {
        return Ok(());
    }

    let batcat_path = command::run("which", &["batcat"])
        .context("bat package installed but batcat is not available")?;
    let local_bin = common::home_dir()?.join(".local").join("bin");
    std::fs::create_dir_all(&local_bin)?;
    let symlink_path = local_bin.join("bat");
    if symlink_path.symlink_metadata().is_ok() {
        bail!(
            "cannot create bat compatibility link because {} already exists but is not usable",
            symlink_path.display()
        );
    }

    std::os::unix::fs::symlink(batcat_path.trim(), &symlink_path)?;
    println!("Created symlink {} -> batcat", symlink_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debian_fallback_without_compatibility_command_is_incomplete() {
        assert!(matches!(
            classify_bat_state(false, true, true),
            InstallationState::Incomplete(_)
        ));
    }
}
