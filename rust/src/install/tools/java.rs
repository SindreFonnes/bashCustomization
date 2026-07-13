use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::{InstallConfig, InstallationState, state_from_components};

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
        missing_java_components().is_empty()
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_debian() || platform.is_fedora() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_fedora()
    }

    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        state_from_components(&[
            ("java", java_component_exists("java")),
            ("javac", java_component_exists("javac")),
        ])
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

fn java_component_exists(name: &str) -> bool {
    if command::run(name, &["-version"]).is_ok() {
        return true;
    }

    if package_manager::has_brew()
        && let Ok(prefix) = command::run("brew", &["--prefix", "openjdk"])
    {
        return std::path::Path::new(&prefix)
            .join("bin")
            .join(name)
            .is_file();
    }

    false
}

fn missing_java_components() -> Vec<&'static str> {
    ["java", "javac"]
        .into_iter()
        .filter(|name| !java_component_exists(name))
        .collect()
}
