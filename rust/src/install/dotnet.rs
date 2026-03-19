use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct DotnetInstaller;

impl super::Installer for DotnetInstaller {
    fn name(&self) -> &str {
        "dotnet"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("dotnet")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install dotnet via brew");
            } else {
                println!("  Would install dotnet-sdk-8.0 via apt (Microsoft repo)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing .NET via brew...");
            return package_manager::brew_install("dotnet");
        }

        install_dotnet_apt(platform)
    }
}

fn install_dotnet_apt(_platform: &Platform) -> Result<()> {
    // Detect distro and version from /etc/os-release
    let (distro_id, version_id) = read_os_release()?;

    println!("Detected distro: {distro_id} {version_id}");

    // Microsoft supports Ubuntu and Debian primarily
    let supported = matches!(
        distro_id.as_str(),
        "ubuntu" | "debian"
    );

    if !supported {
        bail!(
            ".NET SDK apt installation is only supported on Ubuntu and Debian. \
             Detected: {distro_id} {version_id}. Consider using brew or manual installation."
        );
    }

    println!("Adding Microsoft GPG key...");
    package_manager::apt_add_gpg_key(
        "https://packages.microsoft.com/keys/microsoft.asc",
        "/etc/apt/keyrings/microsoft.gpg",
    )?;

    let repo_line = format!(
        "deb [signed-by=/etc/apt/keyrings/microsoft.gpg] https://packages.microsoft.com/{distro_id}/{version_id}/prod {} main",
        get_codename().unwrap_or_else(|| "jammy".to_string())
    );

    println!("Adding Microsoft apt repository...");
    package_manager::apt_add_repo(
        &repo_line,
        "/etc/apt/sources.list.d/microsoft-dotnet.list",
    )?;

    println!("Installing dotnet-sdk-8.0...");
    package_manager::apt_install("dotnet-sdk-8.0")?;

    println!(".NET SDK 8.0 installed");
    Ok(())
}

fn read_os_release() -> Result<(String, String)> {
    let content = std::fs::read_to_string("/etc/os-release")
        .map_err(|_| anyhow::anyhow!("Cannot read /etc/os-release — unable to detect distro"))?;

    let mut id = String::new();
    let mut version_id = String::new();

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("ID=") {
            id = val.trim_matches('"').to_string();
        } else if let Some(val) = line.strip_prefix("VERSION_ID=") {
            version_id = val.trim_matches('"').to_string();
        }
    }

    if id.is_empty() {
        bail!("Could not determine distro ID from /etc/os-release");
    }

    Ok((id, version_id))
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
