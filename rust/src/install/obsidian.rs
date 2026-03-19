use anyhow::{Context, Result};
use serde::Deserialize;

use crate::common::{command, download, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct ObsidianInstaller;

#[derive(Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

impl super::Installer for ObsidianInstaller {
    fn name(&self) -> &str {
        "obsidian"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() // dpkg install needs sudo
    }

    fn is_installed(&self) -> bool {
        command::exists("obsidian")
            || std::path::Path::new("/usr/bin/obsidian").exists()
            || std::path::Path::new("/opt/Obsidian/obsidian").exists()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install Obsidian via brew cask");
            } else {
                println!("  Would download latest Obsidian .deb from GitHub Releases");
            }
            return Ok(());
        }

        if platform.is_mac() {
            println!("Installing Obsidian via brew cask...");
            return package_manager::brew_install_cask("obsidian");
        }

        install_obsidian_deb(platform)
    }
}

fn install_obsidian_deb(platform: &Platform) -> Result<()> {
    println!("Fetching latest Obsidian release...");

    let release: GitHubRelease = download::fetch_json(
        "https://api.github.com/repos/obsidianmd/obsidian-releases/releases/latest",
    )?;

    let arch_suffix = match platform.go_arch() {
        "amd64" => "amd64",
        "arm64" => "arm64",
        other => other,
    };

    let deb_asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".deb") && a.name.contains(arch_suffix))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".deb")))
        .context("no .deb asset found in Obsidian release")?;

    println!("Downloading {}...", deb_asset.name);
    let tmp_dir = std::env::temp_dir();
    let deb_path = tmp_dir.join(&deb_asset.name);

    download::download_file(&deb_asset.browser_download_url, &deb_path)?;

    println!("Installing {}...", deb_asset.name);
    let deb_str = deb_path.to_str().unwrap();
    if command::is_root() {
        command::run_visible("apt-get", &["install", "-y", deb_str])?;
    } else {
        command::run_sudo("apt-get", &["install", "-y", deb_str])?;
    }

    let _ = std::fs::remove_file(&deb_path);
    println!("Obsidian installed");
    Ok(())
}
