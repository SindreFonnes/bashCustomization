use anyhow::Result;

use crate::common::{command, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct JavaScriptInstaller;

impl super::Installer for JavaScriptInstaller {
    fn name(&self) -> &str {
        "javascript"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // yarn via apt needs sudo if no brew
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        // Consider installed if nvm is present (the base dependency)
        std::env::var("NVM_DIR").is_ok()
            && std::path::Path::new(&format!(
                "{}/.nvm/nvm.sh",
                std::env::var("HOME").unwrap_or_default()
            ))
            .exists()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!("  Would install nvm, then pnpm, bun, and yarn");
            return Ok(());
        }

        install_nvm()?;
        install_pnpm()?;
        install_bun()?;
        install_yarn(config)?;

        println!("JavaScript toolchain installed (nvm, pnpm, bun, yarn)");
        Ok(())
    }

    fn phase(&self) -> u8 {
        2 // JS tools must run after other tools, nvm first
    }
}

fn install_nvm() -> Result<()> {
    if command::exists("nvm") || std::path::Path::new(&format!(
        "{}/.nvm/nvm.sh",
        std::env::var("HOME").unwrap_or_default()
    )).exists() {
        println!("nvm already installed, skipping...");
        return Ok(());
    }

    println!("Installing nvm...");
    command::run_visible(
        "bash",
        &[
            "-c",
            "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/master/install.sh | bash",
        ],
    )?;

    // Source nvm and install latest LTS node
    println!("Installing latest Node.js LTS via nvm...");
    command::run_visible(
        "bash",
        &[
            "-c",
            r#"export NVM_DIR="$HOME/.nvm" && [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm install --lts"#,
        ],
    )?;

    Ok(())
}

fn install_pnpm() -> Result<()> {
    if command::exists("pnpm") {
        println!("pnpm already installed, skipping...");
        return Ok(());
    }

    println!("Installing pnpm...");
    command::run_visible(
        "bash",
        &["-c", "curl -fsSL https://get.pnpm.io/install.sh | sh -"],
    )
}

fn install_bun() -> Result<()> {
    if command::exists("bun") {
        println!("bun already installed, skipping...");
        return Ok(());
    }

    println!("Installing bun...");
    command::run_visible(
        "bash",
        &["-c", "curl -fsSL https://bun.sh/install | bash"],
    )
}

fn install_yarn(config: &InstallConfig) -> Result<()> {
    if command::exists("yarn") {
        println!("yarn already installed, skipping...");
        return Ok(());
    }

    if !package_manager::is_brew_failed() && package_manager::has_brew() {
        println!("Installing yarn via brew...");
        return package_manager::brew_install("yarn");
    }

    if config.platform.is_linux() {
        println!("Installing yarn via npm...");
        // Use npm from nvm to install yarn globally
        command::run_visible(
            "bash",
            &[
                "-c",
                r#"export NVM_DIR="$HOME/.nvm" && [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && npm install -g yarn"#,
            ],
        )?;
    }

    Ok(())
}
