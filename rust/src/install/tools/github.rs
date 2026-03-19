use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct GithubCliInstaller;

impl crate::install::Installer for GithubCliInstaller {
    fn name(&self) -> &str {
        "github"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("gh")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
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

        install_github_apt()
    }
}

fn install_github_apt() -> Result<()> {
    println!("Adding GitHub CLI GPG key...");
    package_manager::apt_add_gpg_key(
        "https://cli.github.com/packages/githubcli-archive-keyring.gpg",
        "/etc/apt/keyrings/githubcli-archive-keyring.gpg",
    )?;

    let repo_line = "deb [arch=amd64 signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main";

    println!("Adding GitHub CLI apt repository...");
    package_manager::apt_add_repo(
        repo_line,
        "/etc/apt/sources.list.d/github-cli.list",
    )?;

    println!("Installing gh...");
    package_manager::apt_install("gh")?;

    println!("GitHub CLI installed");
    Ok(())
}
