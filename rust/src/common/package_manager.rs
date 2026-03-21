use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, bail};

use super::command;
use super::platform::{Distro, Platform};
use super::privilege;

static BREW_FAILED: AtomicBool = AtomicBool::new(false);

/// Mark brew as failed for the remainder of this run.
pub fn set_brew_failed() {
    BREW_FAILED.store(true, Ordering::SeqCst);
}

/// Check if brew installation failed earlier in this run.
pub fn is_brew_failed() -> bool {
    BREW_FAILED.load(Ordering::SeqCst)
}

/// Check if brew is available on PATH.
pub fn has_brew() -> bool {
    command::exists("brew")
}

/// Returns true if brew is applicable on this platform.
/// Brew is supported on macOS, Debian, and Fedora. It is not applicable on
/// Arch, Alpine, NixOS, or unknown distros.
fn is_brew_applicable(platform: &Platform) -> bool {
    if platform.is_mac() {
        return true;
    }
    match platform.distro() {
        Some(Distro::Debian) | Some(Distro::Fedora) => true,
        _ => false,
    }
}

/// Ensure Homebrew is installed. On macOS: /opt/homebrew or /usr/local.
/// On Debian/Fedora Linux: Linuxbrew at /home/linuxbrew/.linuxbrew.
/// Skips (no-op) on distros where brew is not applicable (Arch, Alpine, NixOS).
pub fn ensure_brew(platform: &Platform) -> Result<()> {
    if !is_brew_applicable(platform) {
        return Ok(());
    }

    if has_brew() {
        return Ok(());
    }

    if is_brew_failed() {
        bail!("Homebrew installation previously failed this run — skipping");
    }

    println!("Installing Homebrew...");

    if platform.is_mac() {
        // Official Homebrew installer
        let result = command::run_visible(
            "bash",
            &[
                "-c",
                "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
            ],
        );

        if result.is_err() {
            eprintln!("Homebrew installation failed — falling back to native package manager for remaining tools.");
            set_brew_failed();
            return result;
        }

        // Activate brew in current session
        if std::path::Path::new("/opt/homebrew/bin/brew").exists() {
            let shellenv = command::run("/opt/homebrew/bin/brew", &["shellenv"])?;
            command::run_visible("bash", &["-c", &shellenv])?;
        }
    } else if platform.is_linux() {
        let result = command::run_visible(
            "bash",
            &[
                "-c",
                "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
            ],
        );

        if result.is_err() {
            eprintln!("Homebrew installation failed — falling back to native package manager for remaining tools.");
            set_brew_failed();
            return result;
        }

        // Activate Linuxbrew in the current process by parsing `brew shellenv`
        // output and applying env vars directly (running it in a subprocess
        // would only affect that subprocess, not us).
        if std::path::Path::new("/home/linuxbrew/.linuxbrew/bin/brew").exists() {
            let shellenv =
                command::run("/home/linuxbrew/.linuxbrew/bin/brew", &["shellenv"])?;
            for line in shellenv.lines() {
                // Parse lines like: export HOMEBREW_PREFIX="/home/linuxbrew/.linuxbrew"
                if let Some(rest) = line.strip_prefix("export ") {
                    if let Some((key, value)) = rest.split_once('=') {
                        let value = value.trim_matches('"').trim_matches(';');
                        // SAFETY: this runs during single-threaded init
                        // before any parallel tool installs start.
                        unsafe { std::env::set_var(key, value); }
                    }
                }
            }
        }
    }

    if !has_brew() {
        set_brew_failed();
        bail!("Homebrew installation completed but brew is not on PATH");
    }

    Ok(())
}

/// Install a package via brew.
pub fn brew_install(package: &str) -> Result<()> {
    command::run_visible("brew", &["install", package])
}

/// Install a brew cask (macOS only).
pub fn brew_install_cask(package: &str) -> Result<()> {
    command::run_visible("brew", &["install", "--cask", package])
}

/// Install a package using the preferred method for the platform.
///
/// Routing strategy:
/// - macOS → brew
/// - Debian → brew first, apt fallback
/// - Fedora → brew first, dnf fallback
/// - Arch → pacman
/// - Alpine → apk
/// - NixOS → print declarative guidance, return Ok
/// - Unknown → error with distro name and supported list
pub fn install(platform: &Platform, package: &str) -> Result<()> {
    // macOS: brew only
    if platform.is_mac() {
        return brew_install(package);
    }

    // Linux/WSL: route based on distro
    match platform.distro() {
        Some(Distro::Debian) => {
            if !is_brew_failed() && has_brew() {
                return brew_install(package);
            }
            apt_install(package)
        }
        Some(Distro::Fedora) => {
            if !is_brew_failed() && has_brew() {
                return brew_install(package);
            }
            dnf_install(package)
        }
        Some(Distro::Arch) => pacman_install(package),
        Some(Distro::Alpine) => apk_install(package),
        Some(Distro::NixOs) => nix_guidance(package),
        Some(Distro::Unknown(name)) => {
            bail!(
                "Unsupported distro: '{}'. Supported distros: Debian/Ubuntu, \
                 Fedora/RHEL/CentOS, Arch/Manjaro, Alpine, NixOS",
                name
            )
        }
        None => {
            // Should not happen (macOS handled above), but be safe
            bail!("No package manager available to install {package}")
        }
    }
}

/// Install a package via dnf (Fedora/RHEL/CentOS).
/// Stub — not yet implemented.
pub fn dnf_install(package: &str) -> Result<()> {
    bail!(
        "Fedora/RHEL support not yet implemented. Would install: {package}"
    )
}

/// Install a package via pacman (Arch/Manjaro).
/// Stub — not yet implemented.
pub fn pacman_install(package: &str) -> Result<()> {
    bail!(
        "Arch Linux support not yet implemented. Would install: {package}"
    )
}

/// Install a package via apk (Alpine).
/// Stub — not yet implemented.
pub fn apk_install(package: &str) -> Result<()> {
    bail!(
        "Alpine Linux support not yet implemented. Would install: {package}"
    )
}

/// Print declarative guidance for NixOS users.
/// NixOS uses a declarative model; packages are added to configuration, not
/// installed imperatively.
pub fn nix_guidance(package: &str) -> Result<()> {
    println!(
        "NixOS: Add '{package}' to environment.systemPackages in your \
         NixOS configuration, then run `nixos-rebuild switch`."
    );
    Ok(())
}

/// Install a package via apt.
pub fn apt_install(package: &str) -> Result<()> {
    privilege::run_privileged("apt-get", &["install", "-y", package])
}

/// Download a GPG key and install it for apt.
pub fn apt_add_gpg_key(url: &str, keyring_path: &str) -> Result<()> {
    // Ensure /etc/apt/keyrings exists
    privilege::run_privileged("mkdir", &["-p", "/etc/apt/keyrings"])?;

    if url.ends_with(".gpg") {
        // Already a binary keyring — download directly without dearmoring
        let cmd = format!("curl -fsSL '{}' -o '{}'", url, keyring_path);
        privilege::run_privileged("bash", &["-c", &cmd])
    } else {
        // ASCII-armored key (.asc or bare) — download and dearmor
        let cmd = format!(
            "curl -fsSL '{}' | gpg --dearmor -o '{}'",
            url, keyring_path
        );
        privilege::run_privileged("bash", &["-c", &cmd])
    }
}

/// Add an apt repository source file and run apt update.
pub fn apt_add_repo(repo_line: &str, list_file: &str) -> Result<()> {
    let cmd = format!("echo '{}' | tee {}", repo_line, list_file);

    privilege::run_privileged("bash", &["-c", &cmd])?;
    privilege::run_privileged("apt-get", &["update"])
}

/// Returns true if on Linux and not root (needs sudo/privilege escalation
/// for native package operations).
///
/// NixOS never needs sudo for package operations (declarative model).
/// macOS does not use apt, so always returns false.
pub fn needs_sudo_for_native_pkg(platform: &Platform) -> bool {
    if platform.is_mac() {
        return false;
    }
    if platform.is_nixos() {
        return false;
    }
    platform.is_linux() && !command::is_root()
}

/// Legacy alias — prefer `needs_sudo_for_native_pkg`.
pub fn needs_sudo_for_apt(platform: &Platform) -> bool {
    needs_sudo_for_native_pkg(platform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch as CpuArch, Distro, Os, Platform};

    // Helper to build platforms for testing
    fn mac() -> Platform {
        Platform {
            os: Os::MacOs,
            arch: CpuArch::X86_64,
        }
    }
    fn debian() -> Platform {
        Platform {
            os: Os::Linux(Distro::Debian),
            arch: CpuArch::X86_64,
        }
    }
    fn fedora() -> Platform {
        Platform {
            os: Os::Linux(Distro::Fedora),
            arch: CpuArch::X86_64,
        }
    }
    fn arch_linux() -> Platform {
        Platform {
            os: Os::Linux(Distro::Arch),
            arch: CpuArch::X86_64,
        }
    }
    fn alpine() -> Platform {
        Platform {
            os: Os::Linux(Distro::Alpine),
            arch: CpuArch::X86_64,
        }
    }
    fn nixos() -> Platform {
        Platform {
            os: Os::Linux(Distro::NixOs),
            arch: CpuArch::X86_64,
        }
    }
    fn unknown_distro() -> Platform {
        Platform {
            os: Os::Linux(Distro::Unknown("gentoo".to_string())),
            arch: CpuArch::X86_64,
        }
    }
    fn wsl_debian() -> Platform {
        Platform {
            os: Os::Wsl(Distro::Debian),
            arch: CpuArch::X86_64,
        }
    }

    // -----------------------------------------------------------------------
    // is_brew_applicable
    // -----------------------------------------------------------------------

    #[test]
    fn brew_applicable_on_macos() {
        assert!(is_brew_applicable(&mac()));
    }

    #[test]
    fn brew_applicable_on_debian() {
        assert!(is_brew_applicable(&debian()));
    }

    #[test]
    fn brew_applicable_on_fedora() {
        assert!(is_brew_applicable(&fedora()));
    }

    #[test]
    fn brew_not_applicable_on_arch() {
        assert!(!is_brew_applicable(&arch_linux()));
    }

    #[test]
    fn brew_not_applicable_on_alpine() {
        assert!(!is_brew_applicable(&alpine()));
    }

    #[test]
    fn brew_not_applicable_on_nixos() {
        assert!(!is_brew_applicable(&nixos()));
    }

    #[test]
    fn brew_not_applicable_on_unknown() {
        assert!(!is_brew_applicable(&unknown_distro()));
    }

    #[test]
    fn brew_applicable_on_wsl_debian() {
        assert!(is_brew_applicable(&wsl_debian()));
    }

    // -----------------------------------------------------------------------
    // ensure_brew skips on non-applicable distros
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_brew_noop_on_arch() {
        // Should return Ok immediately without trying to install brew
        assert!(ensure_brew(&arch_linux()).is_ok());
    }

    #[test]
    fn ensure_brew_noop_on_alpine() {
        assert!(ensure_brew(&alpine()).is_ok());
    }

    #[test]
    fn ensure_brew_noop_on_nixos() {
        assert!(ensure_brew(&nixos()).is_ok());
    }

    #[test]
    fn ensure_brew_noop_on_unknown() {
        assert!(ensure_brew(&unknown_distro()).is_ok());
    }

    // -----------------------------------------------------------------------
    // Stub functions return appropriate errors
    // -----------------------------------------------------------------------

    #[test]
    fn dnf_install_returns_stub_error() {
        let result = dnf_install("vim");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Fedora/RHEL support not yet implemented"),
            "unexpected error: {msg}"
        );
        assert!(msg.contains("vim"), "error should contain package name: {msg}");
    }

    #[test]
    fn pacman_install_returns_stub_error() {
        let result = pacman_install("vim");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Arch Linux support not yet implemented"),
            "unexpected error: {msg}"
        );
        assert!(msg.contains("vim"), "error should contain package name: {msg}");
    }

    #[test]
    fn apk_install_returns_stub_error() {
        let result = apk_install("vim");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Alpine Linux support not yet implemented"),
            "unexpected error: {msg}"
        );
        assert!(msg.contains("vim"), "error should contain package name: {msg}");
    }

    #[test]
    fn nix_guidance_returns_ok() {
        // nix_guidance should succeed (it just prints advice)
        assert!(nix_guidance("vim").is_ok());
    }

    // -----------------------------------------------------------------------
    // install() routing for NixOS and Unknown
    // -----------------------------------------------------------------------

    #[test]
    fn install_nixos_returns_ok() {
        // NixOS install should print guidance and succeed
        assert!(install(&nixos(), "vim").is_ok());
    }

    #[test]
    fn install_unknown_distro_returns_error() {
        let result = install(&unknown_distro(), "vim");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Unsupported distro"),
            "unexpected error: {msg}"
        );
        assert!(
            msg.contains("gentoo"),
            "error should contain distro name: {msg}"
        );
    }

    #[test]
    fn install_arch_returns_stub_error() {
        let result = install(&arch_linux(), "vim");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Arch Linux support not yet implemented"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn install_alpine_returns_stub_error() {
        let result = install(&alpine(), "vim");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Alpine Linux support not yet implemented"),
            "unexpected error: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // needs_sudo_for_native_pkg
    // -----------------------------------------------------------------------

    #[test]
    fn macos_never_needs_sudo_for_native_pkg() {
        assert!(!needs_sudo_for_native_pkg(&mac()));
    }

    #[test]
    fn nixos_never_needs_sudo_for_native_pkg() {
        assert!(!needs_sudo_for_native_pkg(&nixos()));
    }

    // On the test runner we're not root, so Linux distros should need sudo
    #[test]
    fn debian_needs_sudo_when_not_root() {
        if !command::is_root() {
            assert!(needs_sudo_for_native_pkg(&debian()));
        }
    }

    // -----------------------------------------------------------------------
    // Legacy alias
    // -----------------------------------------------------------------------

    #[test]
    fn needs_sudo_for_apt_matches_native_pkg() {
        let platforms = [mac(), debian(), fedora(), nixos(), arch_linux()];
        for p in &platforms {
            assert_eq!(
                needs_sudo_for_apt(p),
                needs_sudo_for_native_pkg(p),
                "mismatch for platform: {p}"
            );
        }
    }
}
