use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform, privilege};
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
    if !platform.is_debian() {
        let distro = platform.distro();
        bail!(
            "third-party repo setup for docker not yet supported on {:?}",
            distro
        );
    }

    // Determine the distro ID (ubuntu or debian) for the correct Docker repo
    let distro_id = get_os_release_id().unwrap_or_else(|| "ubuntu".to_string());
    let docker_distro = match distro_id.as_str() {
        "debian" => "debian",
        _ => "ubuntu", // Ubuntu and derivatives use the ubuntu repo
    };

    println!("Adding Docker GPG key...");
    let gpg_url = format!("https://download.docker.com/linux/{docker_distro}/gpg");
    package_manager::apt_add_gpg_key(
        &gpg_url,
        "/etc/apt/keyrings/docker.gpg",
    )?;

    let dpkg_arch = platform.go_arch();

    // Detect codename from os-release — fail if missing rather than
    // silently using a wrong default for the wrong distro.
    let codename = get_ubuntu_codename().ok_or_else(|| {
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

fn get_os_release_id() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            return Some(id.trim_matches('"').to_string());
        }
    }
    None
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
