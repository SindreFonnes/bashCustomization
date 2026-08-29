use anyhow::{Context, Result, bail};

use crate::common::{self, command, package_manager, platform::Platform};
use crate::install::{InstallConfig, InstallationState};

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
        canonical_fd_exists()
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform) || platform.is_debian() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        package_manager::is_brew_applicable(platform)
    }

    fn installation_state(&self, platform: &Platform) -> InstallationState {
        classify_fd_state(
            canonical_fd_exists(),
            command::exists("fdfind"),
            platform.is_debian(),
        )
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if package_manager::prefers_brew(&config.platform) {
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
            ensure_fd_compatibility()?;
        }

        Ok(())
    }
}

fn classify_fd_state(
    canonical_exists: bool,
    fallback_exists: bool,
    is_debian: bool,
) -> InstallationState {
    if canonical_exists {
        InstallationState::Complete
    } else if is_debian && fallback_exists {
        InstallationState::Incomplete(
            "fdfind exists but the fd compatibility command is missing".to_string(),
        )
    } else {
        InstallationState::Missing
    }
}

fn fd_compatibility_path() -> Option<std::path::PathBuf> {
    common::home_dir()
        .ok()
        .map(|home| home.join(".local").join("bin").join("fd"))
}

fn canonical_fd_exists() -> bool {
    command::exists("fd")
        || fd_compatibility_path()
            .map(|path| path.exists())
            .unwrap_or(false)
}

fn ensure_fd_compatibility() -> Result<()> {
    if canonical_fd_exists() {
        return Ok(());
    }

    let fdfind_path = command::run("which", &["fdfind"])
        .context("fd-find package installed but fdfind is not available")?;
    let local_bin = common::home_dir()?.join(".local").join("bin");
    std::fs::create_dir_all(&local_bin)?;
    let symlink_path = local_bin.join("fd");
    if symlink_path.symlink_metadata().is_ok() {
        bail!(
            "cannot create fd compatibility link because {} already exists but is not usable",
            symlink_path.display()
        );
    }

    std::os::unix::fs::symlink(fdfind_path.trim(), &symlink_path)?;
    println!("Created symlink {} -> fdfind", symlink_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debian_fallback_without_compatibility_command_is_incomplete() {
        assert!(matches!(
            classify_fd_state(false, true, true),
            InstallationState::Incomplete(_)
        ));
    }
}
