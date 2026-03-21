use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::{self, Platform}, privilege};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct DockerInstaller;

impl crate::install::Installer for DockerInstaller {
    fn name(&self) -> &str {
        "docker"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // On macOS, brew cask doesn't need sudo. On Debian/Ubuntu Linux,
        // the apt-based install always needs root regardless of brew.
        platform.is_debian()
    }

    fn is_installed(&self) -> bool {
        command::exists("docker")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install Docker Desktop via brew cask");
            } else if platform.is_debian() {
                println!("  Would install Docker Engine via apt (GPG key + repo)");
            } else {
                println!("  Docker install not yet supported on this distro");
            }
            return Ok(());
        }

        if platform.is_mac() {
            println!("Installing Docker Desktop via brew...");
            return package_manager::brew_install_cask("docker");
        }

        // On Linux: apt path for Docker Engine (better systemd integration)
        install_docker_apt(platform)
    }
}

fn install_docker_apt(platform: &Platform) -> Result<()> {
    if !platform.is_debian() {
        let distro = platform.distro();
        bail!(
            "third-party repo setup for docker not yet supported on {:?}",
            distro
        );
    }

    // Docker uses separate repo paths for ubuntu vs debian
    let docker_distro = if platform.is_ubuntu() { "ubuntu" } else { "debian" };

    println!("Adding Docker GPG key...");
    let gpg_url = format!("https://download.docker.com/linux/{docker_distro}/gpg");
    package_manager::apt_add_gpg_key(
        &gpg_url,
        "/etc/apt/keyrings/docker.gpg",
    )?;

    let dpkg_arch = platform.go_arch();

    let codename = platform::get_apt_codename().ok_or_else(|| {
        anyhow::anyhow!(
            "could not determine VERSION_CODENAME from /etc/os-release — \
             cannot configure Docker apt repository"
        )
    })?;

    let repo_line = format!(
        "deb [arch={dpkg_arch} signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/{docker_distro} {codename} stable"
    );

    println!("Adding Docker apt repository...");
    package_manager::apt_add_repo(
        &repo_line,
        "/etc/apt/sources.list.d/docker.list",
    )?;

    println!("Installing Docker Engine...");
    let packages = "docker-ce docker-ce-cli containerd.io docker-compose-plugin";
    privilege::run_privileged("apt-get", &["install", "-y",
        "docker-ce", "docker-ce-cli", "containerd.io", "docker-compose-plugin"])?;

    // WSL-specific: create docker group and add user
    if platform.is_wsl() {
        println!("Setting up Docker group for WSL...");
        // Create docker group if it doesn't exist
        let _ = command::run("bash", &["-c", "getent group docker || groupadd docker"]);
        // Add current user to docker group
        if let Ok(user) = std::env::var("USER") {
            let _ = privilege::run_privileged("usermod", &["-aG", "docker", &user]);
            println!("Added {user} to docker group. Log out and back in for changes to take effect.");
        }
    }

    println!("Docker Engine installed ({packages})");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{self, Arch, Distro, Os};
    use crate::install::Installer;

    fn parse_os_release_field(content: &str, key: &str) -> Option<String> {
        platform::parse_os_release_field(content, key.trim_end_matches('='))
    }

    fn debian() -> Platform {
        Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 }
    }
    fn ubuntu() -> Platform {
        Platform { os: Os::Linux(Distro::Ubuntu), arch: Arch::X86_64 }
    }
    fn nixos() -> Platform {
        Platform { os: Os::Linux(Distro::NixOs), arch: Arch::X86_64 }
    }
    fn mac() -> Platform {
        Platform { os: Os::MacOs, arch: Arch::Aarch64 }
    }

    #[test]
    fn needs_sudo_on_debian() {
        assert!(DockerInstaller.needs_sudo(&debian()));
    }

    #[test]
    fn needs_sudo_on_ubuntu() {
        assert!(DockerInstaller.needs_sudo(&ubuntu()));
    }

    #[test]
    fn needs_sudo_false_on_mac() {
        assert!(!DockerInstaller.needs_sudo(&mac()));
    }

    #[test]
    fn needs_sudo_false_on_nixos() {
        assert!(!DockerInstaller.needs_sudo(&nixos()));
    }

    #[test]
    fn parse_os_release_field_ubuntu() {
        let content = "NAME=\"Ubuntu\"\nID=ubuntu\nVERSION_ID=\"22.04\"\nVERSION_CODENAME=jammy\n";
        assert_eq!(parse_os_release_field(content, "ID="), Some("ubuntu".into()));
        assert_eq!(parse_os_release_field(content, "VERSION_CODENAME="), Some("jammy".into()));
    }

    #[test]
    fn parse_os_release_field_debian() {
        let content = "ID=debian\nVERSION_CODENAME=bookworm\n";
        assert_eq!(parse_os_release_field(content, "ID="), Some("debian".into()));
        assert_eq!(parse_os_release_field(content, "VERSION_CODENAME="), Some("bookworm".into()));
    }

    #[test]
    fn parse_os_release_field_missing() {
        let content = "NAME=\"Ubuntu\"\n";
        assert_eq!(parse_os_release_field(content, "ID="), None);
    }

    #[test]
    fn parse_os_release_field_quoted() {
        let content = "ID=\"ubuntu\"\n";
        assert_eq!(parse_os_release_field(content, "ID="), Some("ubuntu".into()));
    }
}
