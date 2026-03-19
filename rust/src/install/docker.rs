use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct DockerInstaller;

impl super::Installer for DockerInstaller {
    fn name(&self) -> &str {
        "docker"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // On macOS, brew doesn't need sudo. On Linux, apt needs sudo.
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("docker")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install Docker Desktop via brew cask");
            } else if package_manager::has_brew() && !package_manager::is_brew_failed() {
                println!("  Would install docker via brew");
            } else {
                println!("  Would install Docker Engine via apt (GPG key + repo)");
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
    println!("Adding Docker GPG key...");
    package_manager::apt_add_gpg_key(
        "https://download.docker.com/linux/ubuntu/gpg",
        "/etc/apt/keyrings/docker.gpg",
    )?;

    let arch = platform.go_arch();
    // Use dpkg arch naming: amd64 or arm64
    let dpkg_arch = match arch {
        "amd64" => "amd64",
        "arm64" => "arm64",
        _ => arch,
    };

    // Detect codename from os-release
    let codename = get_ubuntu_codename().unwrap_or_else(|| "jammy".to_string());

    let repo_line = format!(
        "deb [arch={dpkg_arch} signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu {codename} stable"
    );

    println!("Adding Docker apt repository...");
    package_manager::apt_add_repo(
        &repo_line,
        "/etc/apt/sources.list.d/docker.list",
    )?;

    println!("Installing Docker Engine...");
    let packages = "docker-ce docker-ce-cli containerd.io docker-compose-plugin";
    if command::is_root() {
        command::run_visible("apt-get", &["install", "-y",
            "docker-ce", "docker-ce-cli", "containerd.io", "docker-compose-plugin"])?;
    } else {
        command::run_sudo("apt-get", &["install", "-y",
            "docker-ce", "docker-ce-cli", "containerd.io", "docker-compose-plugin"])?;
    }

    // WSL-specific: create docker group and add user
    if platform.is_wsl() {
        println!("Setting up Docker group for WSL...");
        // Create docker group if it doesn't exist
        let _ = command::run("bash", &["-c", "getent group docker || sudo groupadd docker"]);
        // Add current user to docker group
        if let Ok(user) = std::env::var("USER") {
            let _ = command::run_sudo("usermod", &["-aG", "docker", &user]);
            println!("Added {user} to docker group. Log out and back in for changes to take effect.");
        }
    }

    println!("Docker Engine installed ({packages})");
    Ok(())
}

fn get_ubuntu_codename() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(codename) = line.strip_prefix("VERSION_CODENAME=") {
            return Some(codename.trim_matches('"').to_string());
        }
    }
    None
}
