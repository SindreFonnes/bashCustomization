use anyhow::Result;

use crate::common::{command, download, package_manager, platform::Platform, privilege};
use crate::install::InstallConfig;

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

    let binary_url = format!(
        "https://dl.k8s.io/release/{version}/bin/{go_os}/{go_arch}/kubectl"
    );
    let sha_url = format!("{binary_url}.sha256");

    println!("Downloading kubectl {version}...");
    let tmp_dir = std::env::temp_dir();
    let binary_path = tmp_dir.join("kubectl");

    download::download_file(&binary_url, &binary_path)?;

    println!("Verifying SHA256...");
    let expected_sha = download::fetch_text(&sha_url)?;
    download::verify_sha256(&binary_path, expected_sha.trim())?;
    println!("Checksum OK");

    // Install to /usr/local/bin
    let dest = "/usr/local/bin/kubectl";
    privilege::run_privileged("cp", &[binary_path.to_str().unwrap(), dest])?;
    privilege::run_privileged("chmod", &["+x", dest])?;

    let _ = std::fs::remove_file(&binary_path);
    println!("kubectl {version} installed to {dest}");

    Ok(())
}
