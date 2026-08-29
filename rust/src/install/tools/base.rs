use anyhow::{Context, Result};

use crate::common::{package_manager, platform::Platform, privilege};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct BaseInstaller;

impl crate::install::Installer for BaseInstaller {
    fn name(&self) -> &str {
        "base"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // Base packages on Debian always use apt, which needs root
        platform.is_debian()
    }

    fn is_installed(&self) -> bool {
        false // always run to ensure all base packages are present
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_debian() || platform.is_nixos()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install base packages via brew: git, gnupg");
            } else if platform.is_debian() {
                println!(
                    "  Would install base packages via apt: build-essential, git, safe-rm, keychain, nala, gnupg, etc."
                );
            } else if let Some(distro) = platform.distro() {
                println!("  base packages not yet configured for {distro:?}");
            }
            return Ok(());
        }

        if platform.is_mac() {
            install_base_mac()?;
        } else if platform.is_debian() {
            install_base_linux(platform)?;
        } else if platform.is_nixos() {
            return package_manager::nix_guidance("base development tools");
        } else if platform.is_linux() {
            if let Some(distro) = platform.distro() {
                anyhow::bail!("base packages not yet configured for {distro:?}");
            } else {
                anyhow::bail!("base packages not supported on this platform");
            }
        }

        Ok(())
    }

    fn verify_installation(&self, _platform: &Platform) -> Result<()> {
        // Base installation is an idempotent reconciliation of a package set.
        // Each required package-manager command must succeed; there is no
        // single executable whose presence represents the whole set.
        Ok(())
    }

    fn phase(&self) -> u8 {
        0 // base phase
    }
}

fn install_base_mac() -> Result<()> {
    println!("Installing base packages via brew...");
    let packages = ["git", "gnupg"];
    for pkg in &packages {
        package_manager::brew_install(pkg)
            .with_context(|| format!("installing required base package {pkg}"))?;
    }
    Ok(())
}

fn install_base_linux(platform: &Platform) -> Result<()> {
    // add-apt-repository is provided by software-properties-common. Install it
    // before enabling Ubuntu's universe repository; the full package set below
    // then establishes all Linuxbrew build prerequisites before the Brew phase.
    // The universe repo is Ubuntu-specific; Debian has equivalent packages in main
    if platform.is_ubuntu() {
        privilege::run_privileged("apt-get", &["update"])?;
        privilege::run_privileged("apt-get", &["install", "-y", "software-properties-common"])?;
        println!("Adding universe repository...");
        privilege::run_privileged("add-apt-repository", &["universe", "-y"])
            .context("enabling required Ubuntu universe repository")?;
    }

    let packages = [
        "build-essential",
        "ca-certificates",
        "curl",
        "file",
        "git",
        "safe-rm",
        "keychain",
        "nala",
        "gnupg",
        "pkg-config",
        "procps",
        "libssl-dev",
        "zip",
        "unzip",
        "tar",
        "gzip",
        "net-tools",
        "libfuse2",
        "libnss3-tools",
    ];

    println!("Installing base packages via apt...");
    privilege::run_privileged("apt-get", &["update"])?;

    let mut args = vec!["install", "-y"];
    let pkg_refs: Vec<&str> = packages.to_vec();
    args.extend_from_slice(&pkg_refs);

    privilege::run_privileged("apt-get", &args)?;

    println!("Base packages installed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};
    use crate::install::Installer;

    #[test]
    fn needs_sudo_on_debian() {
        let p = Platform {
            os: Os::Linux(Distro::Debian),
            arch: Arch::X86_64,
        };
        assert!(BaseInstaller.needs_sudo(&p));
    }

    #[test]
    fn needs_sudo_false_on_mac() {
        let p = Platform {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        assert!(!BaseInstaller.needs_sudo(&p));
    }

    #[test]
    fn needs_sudo_false_on_nixos() {
        let p = Platform {
            os: Os::Linux(Distro::NixOs),
            arch: Arch::X86_64,
        };
        assert!(!BaseInstaller.needs_sudo(&p));
    }

    #[test]
    fn is_installed_always_false() {
        assert!(!BaseInstaller.is_installed());
    }

    #[test]
    fn unsupported_distro_errors() {
        let config = crate::install::InstallConfig {
            platform: Platform {
                os: Os::Linux(Distro::Fedora),
                arch: Arch::X86_64,
            },
            dry_run: false,
        };
        let result = BaseInstaller.install(&config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not yet configured")
        );
    }

    #[test]
    fn nixos_returns_guidance() {
        let config = crate::install::InstallConfig {
            platform: Platform {
                os: Os::Linux(Distro::NixOs),
                arch: Arch::X86_64,
            },
            dry_run: false,
        };
        // NixOS guidance returns Ok (prints advice)
        assert!(BaseInstaller.install(&config).is_ok());
    }
}
