use anyhow::{Context, Result};
use serde::Deserialize;

use crate::common::{command, download, package_manager, platform::Platform};
use super::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct GoInstaller;

#[derive(Deserialize)]
struct GoRelease {
    version: String,
    files: Vec<GoFile>,
}

#[derive(Deserialize)]
struct GoFile {
    filename: String,
    os: String,
    arch: String,
    kind: String,
    sha256: String,
}

impl super::Installer for GoInstaller {
    fn name(&self) -> &str {
        "go"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // Needs sudo on Linux for /usr/local/go extraction (only if no brew)
        platform.is_linux() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("go")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install go via brew");
            } else {
                println!("  Would download latest Go from go.dev, verify SHA256, extract to /usr/local/go");
            }
            return Ok(());
        }

        // Try brew first
        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Go via brew...");
            return package_manager::brew_install("go");
        }

        // Fallback: direct download
        install_go_direct(platform)
    }
}

fn install_go_direct(platform: &Platform) -> Result<()> {
    println!("Fetching latest Go release from go.dev...");

    let releases: Vec<GoRelease> =
        download::fetch_json("https://go.dev/dl/?mode=json")
            .context("failed to fetch Go releases")?;

    let release = releases
        .first()
        .context("no Go releases found")?;

    let go_os = platform.go_os();
    let go_arch = platform.go_arch();

    let file = release
        .files
        .iter()
        .find(|f| f.kind == "archive" && f.os == go_os && f.arch == go_arch)
        .with_context(|| {
            format!("no Go archive found for {go_os}/{go_arch} in {}", release.version)
        })?;

    println!(
        "Downloading {} (SHA256: {}...)",
        file.filename,
        &file.sha256[..12]
    );

    let tmp_dir = std::env::temp_dir();
    let archive_path = tmp_dir.join(&file.filename);

    let url = format!("https://go.dev/dl/{}", file.filename);
    download::download_file(&url, &archive_path)?;

    println!("Verifying SHA256...");
    download::verify_sha256(&archive_path, &file.sha256)?;
    println!("Checksum OK");

    // Remove existing Go installation if any
    let go_dir = std::path::Path::new("/usr/local/go");
    if go_dir.exists() {
        if command::is_root() {
            command::run_visible("rm", &["-rf", "/usr/local/go"])?;
        } else {
            command::run_sudo("rm", &["-rf", "/usr/local/go"])?;
        }
    }

    // Extract
    println!("Extracting to /usr/local/go...");
    if command::is_root() {
        command::run_visible(
            "tar",
            &["-C", "/usr/local", "-xzf", archive_path.to_str().unwrap()],
        )?;
    } else {
        command::run_sudo(
            "tar",
            &["-C", "/usr/local", "-xzf", archive_path.to_str().unwrap()],
        )?;
    }

    // Clean up
    let _ = std::fs::remove_file(&archive_path);

    println!("Go {} installed to /usr/local/go", release.version);
    if !std::env::var("PATH").unwrap_or_default().contains("/usr/local/go/bin") {
        println!("Note: Add /usr/local/go/bin to your PATH");
    }

    Ok(())
}
