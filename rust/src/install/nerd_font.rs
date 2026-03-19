use anyhow::{Context, Result};
use serde::Deserialize;

use crate::common::{command, download, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct NerdFontInstaller;

#[derive(Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

impl super::Installer for NerdFontInstaller {
    fn name(&self) -> &str {
        "nerd-font"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false // fonts install to ~/.local/share/fonts
    }

    fn is_installed(&self) -> bool {
        let home = std::env::var("HOME").unwrap_or_default();
        let font_dir = format!("{home}/.local/share/fonts");

        // Check if any JetBrainsMono files exist in the font directory
        if let Ok(entries) = std::fs::read_dir(&font_dir) {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .contains("JetBrainsMono")
                {
                    return true;
                }
            }
        }

        // On macOS, check brew list
        if package_manager::has_brew() {
            return command::run("brew", &["list", "font-jetbrains-mono-nerd-font"])
                .is_ok();
        }

        false
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install font-jetbrains-mono-nerd-font via brew cask");
            } else {
                println!("  Would download JetBrainsMono from GitHub Releases, install to ~/.local/share/fonts");
            }
            return Ok(());
        }

        if platform.is_mac() {
            println!("Installing JetBrains Mono Nerd Font via brew cask...");
            return package_manager::brew_install_cask("font-jetbrains-mono-nerd-font");
        }

        install_nerd_font_linux()
    }
}

fn install_nerd_font_linux() -> Result<()> {
    println!("Fetching latest Nerd Fonts release...");

    let release: GitHubRelease = download::fetch_json(
        "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest",
    )?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "JetBrainsMono.tar.xz")
        .context("JetBrainsMono.tar.xz not found in Nerd Fonts release")?;

    let tmp_dir = std::env::temp_dir();
    let archive_path = tmp_dir.join("JetBrainsMono.tar.xz");

    println!("Downloading JetBrainsMono.tar.xz...");
    download::download_file(&asset.browser_download_url, &archive_path)?;

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let font_dir = format!("{home}/.local/share/fonts/JetBrainsMono");
    std::fs::create_dir_all(&font_dir)?;

    println!("Extracting to {font_dir}...");
    command::run_visible(
        "tar",
        &["-xf", archive_path.to_str().unwrap(), "-C", &font_dir],
    )?;

    let _ = std::fs::remove_file(&archive_path);

    println!("Updating font cache...");
    command::run_visible("fc-cache", &["-fv"])?;

    println!("JetBrains Mono Nerd Font installed");
    Ok(())
}
