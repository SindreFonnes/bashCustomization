use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct AzureInstaller;

impl crate::install::Installer for AzureInstaller {
    fn name(&self) -> &str {
        "azure"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("az")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install azure-cli via brew");
            } else {
                println!("  Would install azure-cli via apt (Microsoft GPG key + repo)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Azure CLI via brew...");
            return package_manager::brew_install("azure-cli");
        }

        install_azure_apt(platform)
    }
}

fn install_azure_apt(platform: &Platform) -> Result<()> {
    if !platform.is_debian() {
        let distro = platform.distro();
        bail!(
            "third-party repo setup for azure not yet supported on {:?}",
            distro
        );
    }

    println!("Adding Microsoft GPG key...");
    package_manager::apt_add_gpg_key(
        "https://packages.microsoft.com/keys/microsoft.asc",
        "/etc/apt/keyrings/microsoft.gpg",
    )?;

    let arch = platform.go_arch();
    let dpkg_arch = match arch {
        "amd64" => "amd64",
        "arm64" => "arm64",
        _ => arch,
    };

    let codename = get_codename().unwrap_or_else(|| "jammy".to_string());

    let repo_line = format!(
        "deb [arch={dpkg_arch} signed-by=/etc/apt/keyrings/microsoft.gpg] https://packages.microsoft.com/repos/azure-cli/ {codename} main"
    );

    println!("Adding Azure CLI apt repository...");
    package_manager::apt_add_repo(
        &repo_line,
        "/etc/apt/sources.list.d/azure-cli.list",
    )?;

    println!("Installing azure-cli...");
    package_manager::apt_install("azure-cli")?;

    println!("Azure CLI installed");
    Ok(())
}

fn get_codename() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(codename) = line.strip_prefix("VERSION_CODENAME=") {
            return Some(codename.trim_matches('"').to_string());
        }
    }
    None
}
