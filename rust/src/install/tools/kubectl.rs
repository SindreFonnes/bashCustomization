use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::common::{command, download, package_manager, platform::Platform, privilege};
use crate::install::{InstallConfig, InstallationState};

#[derive(Debug, Clone, Copy)]
pub struct KubectlInstaller;

impl crate::install::Installer for KubectlInstaller {
    fn name(&self) -> &str {
        "kubectl"
    }

    fn needs_sudo(&self, platform: &Platform) -> bool {
        platform.is_linux() && !platform.is_nixos() && !package_manager::has_brew()
    }

    fn is_installed(&self) -> bool {
        command::exists("kubectl")
    }

    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        if !command::exists("kubectl") {
            InstallationState::Missing
        } else if !package_manager::is_brew_failed()
            && package_manager::has_brew()
            && !command::exists("kubectx")
        {
            InstallationState::Incomplete(
                "kubectl exists but the brew-path kubectx companion is missing".to_string(),
            )
        } else {
            InstallationState::Complete
        }
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if config.dry_run {
            if !package_manager::is_brew_failed() && package_manager::has_brew() {
                println!("  Would install kubernetes-cli and kubectx via brew");
            } else {
                println!("  Would download kubectl binary from dl.k8s.io, verify SHA256");
            }
            return Ok(());
        }

        // Try brew first
        if !package_manager::is_brew_failed() && package_manager::has_brew() {
            println!("Installing kubectl via brew...");
            package_manager::brew_install("kubernetes-cli")?;
            println!("Installing kubectx via brew...");
            package_manager::brew_install("kubectx")?;
            return Ok(());
        }

        // NixOS: emit declarative guidance
        if platform.is_nixos() {
            return package_manager::nix_guidance("kubectl");
        }

        // Fallback: direct download
        install_kubectl_direct(platform)
    }
}

fn install_kubectl_direct(platform: &Platform) -> Result<()> {
    println!("Fetching latest kubectl version...");

    let version = download::fetch_text("https://dl.k8s.io/release/stable.txt")?;
    let version = version.trim();

    let go_os = platform.go_os();
    let go_arch = platform.go_arch();

    let binary_url = format!("https://dl.k8s.io/release/{version}/bin/{go_os}/{go_arch}/kubectl");
    let sha_url = format!("{binary_url}.sha256");

    println!("Downloading kubectl {version}...");
    let temp_dir =
        tempfile::tempdir().context("creating temporary directory for kubectl download")?;
    let binary_path = temp_dir.path().join("kubectl");

    download::download_file(&binary_url, &binary_path)?;

    println!("Verifying SHA256...");
    let expected_sha = download::fetch_text(&sha_url)?;
    download::verify_sha256(&binary_path, expected_sha.trim())?;
    println!("Checksum OK");

    // Stage on the destination filesystem, then atomically replace the final
    // path so a failed copy cannot truncate a working kubectl installation.
    let dest = Path::new("/usr/local/bin/kubectl");
    let staging_id = temp_dir
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .context("temporary kubectl directory name is not valid UTF-8")?;
    install_binary_atomically(&binary_path, dest, staging_id)?;

    let _ = std::fs::remove_file(&binary_path);
    println!("kubectl {version} installed to {}", dest.display());

    Ok(())
}

fn install_binary_atomically(source: &Path, destination: &Path, staging_id: &str) -> Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "kubectl destination has no parent: {}",
            destination.display()
        )
    })?;
    let staged_destination = staged_destination(destination, staging_id)?;

    let source_arg = path_argument(source, "kubectl source")?;
    let parent_arg = path_argument(parent, "kubectl destination directory")?;
    let staged_arg = path_argument(&staged_destination, "kubectl staged destination")?;
    let destination_arg = path_argument(destination, "kubectl destination")?;

    privilege::run_privileged("mkdir", &["-p", parent_arg])?;
    if let Err(install_error) =
        privilege::run_privileged("install", &["-m", "0755", source_arg, staged_arg])
    {
        let cleanup = privilege::run_privileged("rm", &["-f", staged_arg]);
        return Err(with_cleanup_result(
            install_error,
            cleanup,
            &staged_destination,
        ));
    }

    if let Err(move_error) = privilege::run_privileged("mv", &["-f", staged_arg, destination_arg]) {
        let cleanup = privilege::run_privileged("rm", &["-f", staged_arg]);
        return Err(with_cleanup_result(
            move_error,
            cleanup,
            &staged_destination,
        ));
    }

    Ok(())
}

fn path_argument<'a>(path: &'a Path, description: &str) -> Result<&'a str> {
    path.to_str()
        .with_context(|| format!("{description} path is not valid UTF-8: {}", path.display()))
}

fn with_cleanup_result(
    operation_error: anyhow::Error,
    cleanup: Result<()>,
    staged_destination: &Path,
) -> anyhow::Error {
    match cleanup {
        Ok(()) => operation_error.context("atomic kubectl installation failed; staging cleaned up"),
        Err(cleanup_error) => anyhow::anyhow!(
            "Atomic kubectl installation failed: {operation_error:#}. Cleanup also failed for {}: {cleanup_error:#}",
            staged_destination.display()
        ),
    }
}

fn staged_destination(destination: &Path, staging_id: &str) -> Result<PathBuf> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent: {}", destination.display()))?;
    let file_name = destination.file_name().ok_or_else(|| {
        anyhow::anyhow!("destination has no file name: {}", destination.display())
    })?;
    Ok(parent.join(format!(
        ".{}.bashc-stage-{staging_id}",
        file_name.to_string_lossy()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};
    use crate::install::Installer;

    #[test]
    fn needs_sudo_false_on_nixos() {
        let p = Platform {
            os: Os::Linux(Distro::NixOs),
            arch: Arch::X86_64,
        };
        assert!(
            !KubectlInstaller.needs_sudo(&p),
            "NixOS should not need sudo (guidance only)"
        );
    }

    #[test]
    fn needs_sudo_false_on_mac() {
        let p = Platform {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        assert!(!KubectlInstaller.needs_sudo(&p));
    }

    #[test]
    fn stages_next_to_destination_for_atomic_rename() {
        let destination = Path::new("/usr/local/bin/kubectl");
        assert_eq!(
            staged_destination(destination, "tmp123").unwrap(),
            PathBuf::from("/usr/local/bin/.kubectl.bashc-stage-tmp123")
        );
    }
}
