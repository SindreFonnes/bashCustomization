use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

const GITHUB_CLI_APT_KEY_FINGERPRINTS: &[&str] = &[
    "2C6106201985B60E6C7AC87323F3D4EA75716059",
    "7F38BBB59D064DBCB3D84D725612B36462313325",
];

#[derive(Debug, Clone, Copy)]
pub struct GithubCliInstaller;

impl crate::install::Installer for GithubCliInstaller {
    fn name(&self) -> &str {
        "github"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("gh")
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_debian() || platform.is_fedora() || platform.is_nixos()
    }

    fn requires_brew(&self, platform: &Platform) -> bool {
        platform.is_mac() || platform.is_fedora()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install gh via brew");
            } else {
                println!("  Would install gh via apt (GitHub GPG key + repo)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing GitHub CLI via brew...");
            return package_manager::brew_install("gh");
        }

        install_github_apt(platform)
    }
}

fn install_github_apt(platform: &Platform) -> Result<()> {
    if !platform.is_debian() {
        let distro = platform.distro();
        bail!(
            "third-party repo setup for github not yet supported on {:?}",
            distro
        );
    }

    println!("Adding GitHub CLI GPG key...");
    package_manager::apt_add_gpg_key(
        "https://cli.github.com/packages/githubcli-archive-keyring.gpg",
        "/etc/apt/keyrings/githubcli-archive-keyring.gpg",
        GITHUB_CLI_APT_KEY_FINGERPRINTS,
    )?;

    let dpkg_arch = platform.go_arch();
    let repo_line = format!(
        "deb [arch={dpkg_arch} signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main"
    );

    println!("Adding GitHub CLI apt repository...");
    package_manager::apt_add_repo(&repo_line, "/etc/apt/sources.list.d/github-cli.list")?;

    println!("Installing gh...");
    package_manager::apt_install("gh")?;

    println!("GitHub CLI installed");
    Ok(())
}
