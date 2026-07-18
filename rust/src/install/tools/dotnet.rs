use anyhow::{Result, bail};

use crate::common::{
    command, package_manager,
    platform::{self, Platform},
};
use crate::install::{InstallConfig, InstallationState};

const MICROSOFT_APT_KEY_FINGERPRINTS: &[&str] = &["BC528686B50D79E339D3721CEB3E94ADBE1229CF"];

#[derive(Debug, Clone, Copy)]
pub struct DotnetInstaller;

impl crate::install::Installer for DotnetInstaller {
    fn name(&self) -> &str {
        "dotnet"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        dotnet_sdk_installed()
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_mac()
            || (platform.is_debian() && read_os_release().is_ok())
            || platform.is_fedora()
            || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_fedora()
    }

    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        if !command::exists("dotnet") {
            InstallationState::Missing
        } else if dotnet_sdk_installed() {
            InstallationState::Complete
        } else {
            InstallationState::Incomplete(
                "dotnet command exists but no SDK is installed".to_string(),
            )
        }
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

fn dotnet_sdk_installed() -> bool {
    command::run("dotnet", &["--list-sdks"])
        .map(|output| !output.trim().is_empty())
        .unwrap_or(false)
}

fn install_dotnet_apt(platform: &Platform) -> Result<()> {
    if !platform.is_debian() {
        let distro = platform.distro();
        bail!(
            "third-party repo setup for dotnet not yet supported on {:?}",
            distro
        );
    }

    // Detect distro and version from /etc/os-release
    let (distro_id, version_id) = read_os_release()?;

    println!("Detected distro: {distro_id} {version_id}");

    println!("Adding Microsoft GPG key...");
    package_manager::apt_add_gpg_key(
        "https://packages.microsoft.com/keys/microsoft.asc",
        "/etc/apt/keyrings/microsoft.gpg",
        MICROSOFT_APT_KEY_FINGERPRINTS,
    )?;

    let codename = platform::get_apt_codename().ok_or_else(|| {
        anyhow::anyhow!(
            "could not determine VERSION_CODENAME from /etc/os-release — \
             cannot configure .NET SDK apt repository"
        )
    })?;

    let repo_line = format!(
        "deb [signed-by=/etc/apt/keyrings/microsoft.gpg] https://packages.microsoft.com/{distro_id}/{version_id}/prod {codename} main"
    );

    println!("Adding Microsoft apt repository...");
    package_manager::apt_add_repo(&repo_line, "/etc/apt/sources.list.d/microsoft-dotnet.list")?;

    println!("Installing dotnet-sdk-8.0...");
    package_manager::apt_install("dotnet-sdk-8.0")?;

    println!(".NET SDK 8.0 installed");
    Ok(())
}

fn read_os_release() -> Result<(String, String)> {
    let content = std::fs::read_to_string("/etc/os-release")
        .map_err(|_| anyhow::anyhow!("Cannot read /etc/os-release — unable to detect distro"))?;
    parse_os_release_content(&content)
}

/// Parse ID and VERSION_ID from os-release content.
fn parse_os_release_content(content: &str) -> Result<(String, String)> {
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

    if version_id.is_empty() {
        bail!(
            "Could not determine VERSION_ID from /etc/os-release — needed for Microsoft repo URL"
        );
    }

    if id != "ubuntu" && id != "debian" {
        bail!(
            "Microsoft's .NET apt repository is supported only on exact Ubuntu or Debian hosts; detected '{id}'. Debian-family derivatives need an explicit mapping to their parent repository coordinates"
        );
    }

    Ok((id, version_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ubuntu_os_release() {
        let content = "ID=ubuntu\nVERSION_ID=\"22.04\"\n";
        let (id, ver) = parse_os_release_content(content).unwrap();
        assert_eq!(id, "ubuntu");
        assert_eq!(ver, "22.04");
    }

    #[test]
    fn parse_debian_os_release() {
        let content = "ID=debian\nVERSION_ID=\"12\"\n";
        let (id, ver) = parse_os_release_content(content).unwrap();
        assert_eq!(id, "debian");
        assert_eq!(ver, "12");
    }

    #[test]
    fn parse_missing_id_errors() {
        let content = "VERSION_ID=\"22.04\"\n";
        let result = parse_os_release_content(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("distro ID"));
    }

    #[test]
    fn parse_missing_version_id_errors() {
        let content = "ID=ubuntu\n";
        let result = parse_os_release_content(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("VERSION_ID"));
    }

    #[test]
    fn parse_empty_version_id_errors() {
        let content = "ID=ubuntu\nVERSION_ID=\n";
        let result = parse_os_release_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_debian_derivative_errors_before_repo_setup() {
        let content = "ID=linuxmint\nID_LIKE=\"ubuntu debian\"\nVERSION_ID=\"22\"\n";
        let error = parse_os_release_content(content).unwrap_err();
        assert!(error.to_string().contains("exact Ubuntu or Debian"));
        assert!(error.to_string().contains("linuxmint"));
    }
}
