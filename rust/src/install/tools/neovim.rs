use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::os::unix::fs::PermissionsExt;

use crate::common::{
    self, command, download, package_manager,
    platform::{Arch, Platform},
};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct NeovimInstaller;

#[derive(Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

impl crate::install::Installer for NeovimInstaller {
    fn name(&self) -> &str {
        "neovim"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        command::exists("nvim")
            || common::home_dir()
                .map(|home| home.join(".mybin").join("nvim").is_file())
                .unwrap_or(false)
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install neovim via brew");
            } else {
                println!(
                    "  Would download the architecture-specific Neovim AppImage, verify its GitHub SHA-256 digest, and atomically install it to ~/.mybin/nvim"
                );
            }
            return Ok(());
        }

        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Neovim via brew...");
            return package_manager::brew_install("neovim");
        }

        if platform.is_linux() {
            return install_neovim_appimage(platform.arch);
        }

        bail!("Neovim installation is not supported on {platform}")
    }
}

fn install_neovim_appimage(architecture: Arch) -> Result<()> {
    let architecture = match architecture {
        Arch::X86_64 => "x86_64",
        Arch::Aarch64 => "arm64",
    };
    let asset_name = format!("nvim-linux-{architecture}.appimage");
    let release: GitHubRelease =
        download::fetch_json("https://api.github.com/repos/neovim/neovim/releases/latest")?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .with_context(|| format!("Neovim release did not contain {asset_name}"))?;

    println!("Downloading {asset_name}...");

    let mybin = dirs_mybin()?;
    let dest = mybin.join("nvim");

    // Stage on the destination filesystem so the final rename is atomic.
    let temp_dir = tempfile::tempdir_in(&mybin)
        .context("creating same-filesystem staging directory for Neovim")?;
    let temp_dest = temp_dir.path().join("nvim");

    download::download_file(&asset.browser_download_url, &temp_dest)?;
    download::verify_github_asset_digest(&temp_dest, asset.digest.as_deref())?;
    std::fs::set_permissions(&temp_dest, std::fs::Permissions::from_mode(0o755))
        .context("making staged Neovim AppImage executable")?;
    std::fs::File::open(&temp_dest)?.sync_all()?;
    std::fs::rename(&temp_dest, &dest)
        .with_context(|| format!("atomically replacing {}", dest.display()))?;

    println!("Neovim AppImage installed to {}", dest.display());
    Ok(())
}

fn dirs_mybin() -> Result<std::path::PathBuf> {
    let mybin = common::home_dir()?.join(".mybin");
    std::fs::create_dir_all(&mybin)?;
    Ok(mybin)
}
