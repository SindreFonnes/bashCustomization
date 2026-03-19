use anyhow::Result;

use crate::common::{command, download, package_manager, platform::Platform};
use super::InstallConfig;

pub struct KubectlInstaller;

impl super::Installer for KubectlInstaller {
    fn name(&self) -> &str {
        "kubectl"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
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
    if command::is_root() {
        command::run_visible("cp", &[binary_path.to_str().unwrap(), dest])?;
        command::run_visible("chmod", &["+x", dest])?;
    } else {
        command::run_sudo("cp", &[binary_path.to_str().unwrap(), dest])?;
        command::run_sudo("chmod", &["+x", dest])?;
    }

    let _ = std::fs::remove_file(&binary_path);
    println!("kubectl {version} installed to {dest}");

    Ok(())
}
