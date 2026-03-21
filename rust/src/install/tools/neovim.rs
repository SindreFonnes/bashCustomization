use anyhow::{Result, bail};

use crate::common::{command, download, package_manager, platform::{Arch, Platform}};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct NeovimInstaller;

impl crate::install::Installer for NeovimInstaller {
    fn name(&self) -> &str {
        "neovim"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // Needs sudo on aarch64 Debian Linux without brew (apt install)
        platform.is_debian()
            && !package_manager::has_brew()
            && platform.arch == Arch::Aarch64
    }

    fn is_installed(&self) -> bool {
        command::exists("nvim")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install neovim via brew");
            } else if platform.arch == Arch::X86_64 {
                println!("  Would download nvim.appimage to ~/.mybin/nvim");
            } else {
                println!("  Would install neovim via apt");
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Neovim via brew...");
            return package_manager::brew_install("neovim");
        }

        if platform.is_linux() && platform.arch == Arch::X86_64 {
            return install_neovim_appimage();
        }

        // aarch64 Linux: apt (Debian only)
        if !platform.is_debian() {
            bail!(
                "Neovim aarch64 install via apt is only supported on Debian-based distros. \
                 On other distros, install brew first or install neovim manually."
            );
        }
        println!("Installing Neovim via apt...");
        package_manager::apt_install("neovim")
    }
}

fn install_neovim_appimage() -> Result<()> {
    println!("Downloading Neovim AppImage...");

    let mybin = dirs_mybin()?;
    let dest = mybin.join("nvim");

    download::download_file(
        "https://github.com/neovim/neovim/releases/latest/download/nvim.appimage",
        &dest,
    )?;

    // Make executable
    command::run_visible("chmod", &["+x", dest.to_str().unwrap()])?;

    println!("Neovim AppImage installed to {}", dest.display());
    Ok(())
}

fn dirs_mybin() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let mybin = std::path::PathBuf::from(home).join(".mybin");
    std::fs::create_dir_all(&mybin)?;
    Ok(mybin)
}
