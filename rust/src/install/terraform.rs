use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

pub struct TerraformInstaller;

impl super::Installer for TerraformInstaller {
    fn name(&self) -> &str {
        "terraform"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("terraform")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install terraform via brew");
            } else {
                println!("  Would install terraform via apt (HashiCorp GPG key + repo)");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Terraform via brew...");
            return package_manager::brew_install("terraform");
        }

        install_terraform_apt()
    }
}

fn install_terraform_apt() -> Result<()> {
    println!("Adding HashiCorp GPG key...");
    package_manager::apt_add_gpg_key(
        "https://apt.releases.hashicorp.com/gpg",
        "/etc/apt/keyrings/hashicorp.gpg",
    )?;

    let codename = get_codename().unwrap_or_else(|| "jammy".to_string());

    let repo_line = format!(
        "deb [signed-by=/etc/apt/keyrings/hashicorp.gpg] https://apt.releases.hashicorp.com {codename} main"
    );

    println!("Adding HashiCorp apt repository...");
    package_manager::apt_add_repo(
        &repo_line,
        "/etc/apt/sources.list.d/hashicorp.list",
    )?;

    println!("Installing terraform...");
    package_manager::apt_install("terraform")?;

    println!("Terraform installed");
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
