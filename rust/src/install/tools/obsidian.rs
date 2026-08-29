use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::common::{
    command, download, package_manager,
    platform::{Arch, Platform},
    privilege,
};
use crate::install::InstallConfig;

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
    digest: Option<String>,
}

impl crate::install::Installer for ObsidianInstaller {
    fn name(&self) -> &str {
        "obsidian"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_debian() // dpkg/apt install needs sudo
    }

    fn is_installed(&self) -> bool {
        command::exists("obsidian")
            || std::path::Path::new("/usr/bin/obsidian").exists()
            || std::path::Path::new("/opt/Obsidian/obsidian").exists()
            || std::path::Path::new("/Applications/Obsidian.app").is_dir()
            || crate::common::home_dir()
                .map(|home| home.join("Applications/Obsidian.app").is_dir())
                .unwrap_or(false)
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_mac()
            || (platform.is_debian() && platform.arch == Arch::X86_64)
            || platform.is_nixos()
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

        if !platform.is_debian() {
            bail!("Obsidian .deb install only supported on Debian-based distros");
        }

        install_obsidian_deb(platform)
    }
}

fn install_obsidian_deb(platform: &Platform) -> Result<()> {
    println!("Fetching latest Obsidian release...");

    let release: GitHubRelease = download::fetch_json(
        "https://api.github.com/repos/obsidianmd/obsidian-releases/releases/latest",
    )?;

    let arch_suffix = platform.go_arch();

    let deb_asset = select_deb_asset(&release.assets, arch_suffix).with_context(|| {
        let available = release
            .assets
            .iter()
            .filter(|asset| asset.name.ends_with(".deb"))
            .map(|asset| asset.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "no Obsidian .deb asset found for architecture {arch_suffix}; available .deb assets: {available}"
        )
    })?;

    println!("Downloading {}...", deb_asset.name);
    let temp_dir =
        tempfile::tempdir().context("creating temporary directory for Obsidian download")?;
    let deb_path = temp_dir.path().join(&deb_asset.name);

    download::download_file(&deb_asset.browser_download_url, &deb_path)?;
    download::verify_github_asset_digest(&deb_path, deb_asset.digest.as_deref())?;

    println!("Installing {}...", deb_asset.name);
    let deb_str = deb_path.to_str().with_context(|| {
        format!(
            "Obsidian package path is not valid UTF-8: {}",
            deb_path.display()
        )
    })?;
    privilege::run_privileged("apt-get", &["install", "-y", deb_str])?;

    let _ = std::fs::remove_file(&deb_path);
    println!("Obsidian installed");
    Ok(())
}

fn select_deb_asset<'a>(assets: &'a [GitHubAsset], architecture: &str) -> Option<&'a GitHubAsset> {
    assets.iter().find(|asset| {
        let Some(stem) = asset.name.strip_suffix(".deb") else {
            return false;
        };
        stem.split(['-', '_'])
            .any(|component| component == architecture)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.invalid/{name}"),
            digest: None,
        }
    }

    #[test]
    fn selects_only_the_requested_deb_architecture() {
        let assets = vec![
            asset("obsidian_1.0.0_amd64.deb"),
            asset("obsidian_1.0.0_arm64.deb"),
        ];

        assert_eq!(
            select_deb_asset(&assets, "arm64").unwrap().name,
            assets[1].name
        );
    }

    #[test]
    fn does_not_fall_back_to_a_generic_or_wrong_architecture_deb() {
        let assets = vec![
            asset("obsidian_1.0.0_amd64.deb"),
            asset("obsidian_latest.deb"),
        ];

        assert!(select_deb_asset(&assets, "arm64").is_none());
    }
}
