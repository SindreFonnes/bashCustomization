use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::common::{command, download, package_manager, platform::Platform, privilege};
use crate::install::InstallConfig;

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

impl crate::install::Installer for GoInstaller {
    fn name(&self) -> &str {
        "go"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        // Needs sudo on Linux for /usr/local/go extraction (only if no brew).
        // NixOS emits guidance only, no root needed.
        platform.is_linux() && !platform.is_nixos() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("go") || std::path::Path::new("/usr/local/go/bin/go").is_file()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install go via brew");
            } else {
                println!(
                    "  Would download latest Go from go.dev, verify SHA256, extract to /usr/local/go"
                );
            }
            return Ok(());
        }

        // Try brew first
        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing Go via brew...");
            return package_manager::brew_install("go");
        }

        // NixOS: emit declarative guidance
        if platform.is_nixos() {
            return package_manager::nix_guidance("go");
        }

        // Fallback: direct download
        install_go_direct(platform)
    }
}

fn install_go_direct(platform: &Platform) -> Result<()> {
    command::require("tar")?;
    println!("Fetching latest Go release from go.dev...");

    let releases: Vec<GoRelease> = download::fetch_json("https://go.dev/dl/?mode=json")
        .context("failed to fetch Go releases")?;

    let release = releases.first().context("no Go releases found")?;

    let go_os = platform.go_os();
    let go_arch = platform.go_arch();

    let file = release
        .files
        .iter()
        .find(|f| f.kind == "archive" && f.os == go_os && f.arch == go_arch)
        .with_context(|| {
            format!(
                "no Go archive found for {go_os}/{go_arch} in {}",
                release.version
            )
        })?;

    println!(
        "Downloading {} (SHA256: {}...)",
        file.filename,
        file.sha256.get(..12).unwrap_or(&file.sha256)
    );

    let temp_dir = tempfile::tempdir().context("creating temporary directory for Go download")?;
    let archive_path = temp_dir.path().join(&file.filename);

    let url = format!("https://go.dev/dl/{}", file.filename);
    download::download_file(&url, &archive_path)?;

    println!("Verifying SHA256...");
    download::verify_sha256(&archive_path, &file.sha256)?;
    println!("Checksum OK");

    // Extract and validate without touching the working installation.
    let extracted_root = temp_dir.path().join("extracted");
    std::fs::create_dir(&extracted_root)?;
    println!("Extracting and validating staged Go distribution...");
    let extracted_root_arg = path_arg(&extracted_root)?;
    let archive_arg = path_arg(&archive_path)?;
    command::run_visible("tar", &["-C", extracted_root_arg, "-xzf", archive_arg])?;
    let extracted_go = extracted_root.join("go");
    if !extracted_go.join("bin/go").is_file() {
        bail!("downloaded Go archive did not contain go/bin/go")
    }

    replace_go_installation(&extracted_go)?;

    // Clean up
    let _ = std::fs::remove_file(&archive_path);

    println!("Go {} installed to /usr/local/go", release.version);
    if !std::env::var("PATH")
        .unwrap_or_default()
        .contains("/usr/local/go/bin")
    {
        println!("Note: Add /usr/local/go/bin to your PATH");
    }

    Ok(())
}

fn replace_go_installation(extracted_go: &std::path::Path) -> Result<()> {
    let process_id = std::process::id();
    let destination = std::path::Path::new("/usr/local/go");
    let staged = std::path::PathBuf::from(format!("/usr/local/.bashc-go-stage-{process_id}"));
    let backup = std::path::PathBuf::from(format!("/usr/local/.bashc-go-backup-{process_id}"));

    if staged.symlink_metadata().is_ok() || backup.symlink_metadata().is_ok() {
        bail!(
            "refusing to reuse existing Go transaction paths {} or {}; remove the stale path after inspecting it",
            staged.display(),
            backup.display()
        )
    }

    let extracted = path_arg(extracted_go)?;
    let staged_arg = path_arg(&staged)?;
    let backup_arg = path_arg(&backup)?;

    println!("Staging Go distribution on /usr/local...");
    if let Err(error) = privilege::run_privileged("cp", &["-a", "--", extracted, staged_arg]) {
        let _ = privilege::run_privileged("rm", &["-rf", "--", staged_arg]);
        return Err(error).context("copying staged Go distribution to /usr/local");
    }
    if let Err(error) = privilege::run_privileged("chown", &["-R", "root:root", staged_arg]) {
        let _ = privilege::run_privileged("rm", &["-rf", "--", staged_arg]);
        return Err(error).context("setting ownership on staged Go distribution");
    }

    let had_previous = destination.symlink_metadata().is_ok();
    if had_previous
        && let Err(error) = privilege::run_privileged("mv", &["--", "/usr/local/go", backup_arg])
    {
        let _ = privilege::run_privileged("rm", &["-rf", "--", staged_arg]);
        return Err(error).context("moving the previous Go installation aside");
    }

    if let Err(install_error) =
        privilege::run_privileged("mv", &["--", staged_arg, "/usr/local/go"])
    {
        if had_previous {
            privilege::run_privileged("mv", &["--", backup_arg, "/usr/local/go"])
                .with_context(|| {
                    format!(
                        "activating staged Go failed ({install_error:#}); restoring the previous installation also failed"
                    )
                })?;
        }
        let _ = privilege::run_privileged("rm", &["-rf", "--", staged_arg]);
        return Err(install_error).context("activating staged Go distribution");
    }

    if had_previous && let Err(error) = privilege::run_privileged("rm", &["-rf", "--", backup_arg])
    {
        eprintln!(
            "Warning: Go was upgraded, but the previous installation could not be removed from {}: {error:#}",
            backup.display()
        );
    }

    Ok(())
}

fn path_arg(path: &std::path::Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("path is not valid UTF-8: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};
    use crate::install::Installer;

    #[test]
    fn needs_sudo_on_debian_without_brew() {
        let p = Platform {
            os: Os::Linux(Distro::Debian),
            arch: Arch::X86_64,
        };
        // May be true or false depending on whether brew is on PATH in test env,
        // but should not panic
        let _ = GoInstaller.needs_sudo(&p);
    }

    #[test]
    fn needs_sudo_false_on_nixos() {
        let p = Platform {
            os: Os::Linux(Distro::NixOs),
            arch: Arch::X86_64,
        };
        assert!(
            !GoInstaller.needs_sudo(&p),
            "NixOS should not need sudo (guidance only)"
        );
    }

    #[test]
    fn needs_sudo_false_on_mac() {
        let p = Platform {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        assert!(!GoInstaller.needs_sudo(&p));
    }
}
