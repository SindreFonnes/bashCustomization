use anyhow::{Context, Result};
use serde::Deserialize;

use crate::common::{self, command, download, package_manager, platform::Platform};
use crate::install::InstallConfig;

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

impl crate::install::Installer for NerdFontInstaller {
    fn name(&self) -> &str {
        "nerd-font"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false // fonts install to ~/.local/share/fonts
    }

    fn is_installed(&self) -> bool {
        let Ok(home) = common::home_dir() else {
            return false;
        };
        let font_dir = home.join(".local").join("share").join("fonts");

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
            return command::run("brew", &["list", "font-jetbrains-mono-nerd-font"]).is_ok();
        }

        false
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if platform.is_mac() {
                println!("  Would install font-jetbrains-mono-nerd-font via brew cask");
            } else {
                println!(
                    "  Would download JetBrainsMono from GitHub Releases, install to ~/.local/share/fonts"
                );
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
    command::require_all(&["tar", "fc-cache"])?;
    println!("Fetching latest Nerd Fonts release...");

    let release: GitHubRelease =
        download::fetch_json("https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest")?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "JetBrainsMono.tar.xz")
        .context("JetBrainsMono.tar.xz not found in Nerd Fonts release")?;
    let checksum_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA-256.txt")
        .context("SHA-256.txt not found in Nerd Fonts release")?;

    let temp_dir =
        tempfile::tempdir().context("creating temporary directory for Nerd Font download")?;
    let archive_path = temp_dir.path().join("JetBrainsMono.tar.xz");

    println!("Downloading JetBrainsMono.tar.xz...");
    download::download_file(&asset.browser_download_url, &archive_path)?;
    let checksum_manifest = download::fetch_text(&checksum_asset.browser_download_url)?;
    let expected = download::sha256_from_manifest(&checksum_manifest, "JetBrainsMono.tar.xz")?;
    download::verify_sha256(&archive_path, &expected)?;

    let font_root = common::home_dir()?
        .join(".local")
        .join("share")
        .join("fonts");
    let font_dir = font_root.join("JetBrainsMono");
    std::fs::create_dir_all(&font_root)?;

    // Extract and validate the complete replacement before touching the
    // currently installed font directory.
    let staging = tempfile::tempdir_in(&font_root)
        .context("creating same-filesystem staging directory for Nerd Font")?;
    let staged_font_dir = staging.path().join("new");
    std::fs::create_dir(&staged_font_dir)?;

    println!("Extracting staged font files...");
    let archive_arg = archive_path.to_str().with_context(|| {
        format!(
            "Nerd Font archive path is not valid UTF-8: {}",
            archive_path.display()
        )
    })?;
    let staged_font_arg = staged_font_dir.to_str().with_context(|| {
        format!(
            "Nerd Font staging path is not valid UTF-8: {}",
            staged_font_dir.display()
        )
    })?;
    command::run_visible("tar", &["-xf", archive_arg, "-C", staged_font_arg])?;

    if !contains_font_file(&staged_font_dir)? {
        anyhow::bail!("downloaded Nerd Font archive did not contain any .ttf or .otf files");
    }

    let backup = staging.path().join("previous");
    let had_previous = font_dir.symlink_metadata().is_ok();
    if had_previous {
        std::fs::rename(&font_dir, &backup)
            .with_context(|| format!("staging previous font directory {}", font_dir.display()))?;
    }

    if let Err(error) = std::fs::rename(&staged_font_dir, &font_dir) {
        if had_previous {
            std::fs::rename(&backup, &font_dir).with_context(|| {
                format!(
                    "installing staged fonts failed ({error}); restoring previous font directory also failed"
                )
            })?;
        }
        return Err(error).with_context(|| format!("replacing {}", font_dir.display()));
    }

    let _ = std::fs::remove_file(&archive_path);

    println!("Updating font cache...");
    command::run_visible("fc-cache", &["-fv"])?;

    println!("JetBrains Mono Nerd Font installed");
    Ok(())
}

fn contains_font_file(directory: &std::path::Path) -> Result<bool> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if contains_font_file(&path)? {
                return Ok(true);
            }
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("ttf") || extension.eq_ignore_ascii_case("otf")
            })
        {
            return Ok(true);
        }
    }

    Ok(false)
}
