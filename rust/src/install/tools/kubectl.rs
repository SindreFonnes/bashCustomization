use anyhow::{Context, Result};

use crate::common::{command, download, package_manager, platform::Platform, privilege};
use crate::install::{InstallConfig, InstallationState};

#[derive(Debug, Clone, Copy)]
pub struct KubectlInstaller;

impl crate::install::Installer for KubectlInstaller {
    fn name(&self) -> &str {
        "kubectl"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !platform.is_nixos() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("kubectl")
    }

    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        if !command::exists("kubectl") {
            InstallationState::Missing
        } else if !package_manager::is_brew_failed()
            && package_manager::has_brew()
            && !command::exists("kubectx")
        {
            InstallationState::Incomplete(
                "kubectl exists but the brew-path kubectx companion is missing".to_string(),
            )
        } else {
            InstallationState::Complete
        }
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install kubernetes-cli and kubectx via brew");
            } else {
                println!("  Would download kubectl binary from dl.k8s.io, verify SHA256");
            }
            return Ok(());
        }

        // Try brew first
        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing kubectl via brew...");
            package_manager::brew_install("kubernetes-cli")?;
            println!("Installing kubectx via brew...");
            package_manager::brew_install("kubectx")?;
            return Ok(());
        }

        // NixOS: emit declarative guidance
        if platform.is_nixos() {
            return package_manager::nix_guidance("kubectl");
        }

        // Fallback: direct download
        install_kubectl_direct(platform)
    }
}

fn install_kubectl_direct(platform: &Platform) -> Result<()> {
    println!("Fetching latest kubectl version...");

    let version = download::fetch_text("https://dl.k8s.io/release/stable.txt")?;
    let version = version.trim();

    let go_os = platform.go_os();
    let go_arch = platform.go_arch();

    let binary_url = format!("https://dl.k8s.io/release/{version}/bin/{go_os}/{go_arch}/kubectl");
    let sha_url = format!("{binary_url}.sha256");

    println!("Downloading kubectl {version}...");
    let temp_dir =
        tempfile::tempdir().context("creating temporary directory for kubectl download")?;
    let binary_path = temp_dir.path().join("kubectl");

    download::download_file(&binary_url, &binary_path)?;

    println!("Verifying SHA256...");
    let expected_sha = download::fetch_text(&sha_url)?;
    download::verify_sha256(&binary_path, expected_sha.trim())?;
    println!("Checksum OK");

    // Install to /usr/local/bin
    let dest = "/usr/local/bin/kubectl";
    privilege::run_privileged(
        "install",
        &[
            "-D",
            "-m",
            "0755",
            "--",
            binary_path.to_str().unwrap(),
            dest,
        ],
    )?;

    let _ = std::fs::remove_file(&binary_path);
    println!("kubectl {version} installed to {dest}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};
    use crate::install::Installer;

    #[test]
    fn needs_sudo_false_on_nixos() {
        let p = Platform {
            os: Os::Linux(Distro::NixOs),
            arch: Arch::X86_64,
        };
        assert!(
            !KubectlInstaller.needs_sudo(&p),
            "NixOS should not need sudo (guidance only)"
        );
    }

    #[test]
    fn needs_sudo_false_on_mac() {
        let p = Platform {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        assert!(!KubectlInstaller.needs_sudo(&p));
    }
}
