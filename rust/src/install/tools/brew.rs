use anyhow::Result;

use crate::common::{package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct BrewInstaller;

impl crate::install::Installer for BrewInstaller {
    fn name(&self) -> &str {
        "brew"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        package_manager::has_brew()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!("  Would install Homebrew");
            return Ok(());
        }

        if !package_manager::is_brew_applicable(&config.platform) {
            anyhow::bail!(
                "Homebrew is not supported on {:?}",
                config.platform.distro()
            );
        }

        println!("Installing Homebrew...");
        package_manager::ensure_brew(&config.platform)
    }

    fn phase(&self) -> u8 {
        0 // base phase — must run before other installers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};
    use crate::install::Installer;

    #[test]
    fn needs_sudo_always_false() {
        let platforms = [
            Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 },
            Platform { os: Os::MacOs, arch: Arch::Aarch64 },
            Platform { os: Os::Linux(Distro::NixOs), arch: Arch::X86_64 },
        ];
        for p in &platforms {
            assert!(!BrewInstaller.needs_sudo(p));
        }
    }

    #[test]
    fn unsupported_distro_errors() {
        let config = crate::install::InstallConfig {
            platform: Platform { os: Os::Linux(Distro::Alpine), arch: Arch::X86_64 },
            dry_run: false,
            verbose: false,
            interactive: false,
        };
        let result = BrewInstaller.install(&config);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not supported"),
            "should mention brew not supported"
        );
    }
}
