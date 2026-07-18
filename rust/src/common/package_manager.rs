use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};

use super::command;
use super::download;
use super::platform::{Distro, Platform};
use super::privilege;

static BREW_FAILED: AtomicBool = AtomicBool::new(false);

const HOMEBREW_INSTALL_URL: &str =
    "https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh";
const HOMEBREW_INSTALL_SHA256: &str =
    "99287f194a8b3c9e6b0203a11a5fa54518be57209343e6bb954dec4635796d9d";

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
pub fn is_brew_applicable(platform: &Platform) -> bool {
    if platform.is_mac() {
        return true;
    }
    matches!(
        platform.distro(),
        Some(Distro::Debian | Distro::Ubuntu | Distro::Fedora)
    )
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

    // Homebrew may already be installed at its standard prefix without that
    // prefix being active in this process (especially on Apple Silicon and
    // fresh Linux installs). Activate it before attempting another install.
    if let Some(brew) = known_brew_executable(platform) {
        return activate_homebrew(&brew);
    }

    if is_brew_failed() {
        bail!("Homebrew installation previously failed this run — skipping");
    }

    println!("Installing Homebrew...");
    command::require_all(&["bash", "curl"])
        .context("Homebrew bootstrap prerequisites are not satisfied")?;

    let result = download::run_verified_script(
        HOMEBREW_INSTALL_URL,
        HOMEBREW_INSTALL_SHA256,
        "env",
        &["NONINTERACTIVE=1", "/bin/bash"],
        &[],
    );

    if result.is_err() {
        eprintln!(
            "Homebrew installation failed — falling back to native package manager for remaining tools."
        );
        set_brew_failed();
        return result;
    }

    let Some(brew) = known_brew_executable(platform) else {
        set_brew_failed();
        bail!(
            "Homebrew installation completed but no brew executable was found at a supported prefix"
        );
    };

    if let Err(error) = activate_homebrew(&brew) {
        set_brew_failed();
        return Err(error).context("Homebrew installed but could not be activated");
    }

    Ok(())
}

fn known_brew_executable(platform: &Platform) -> Option<PathBuf> {
    let candidates: &[&str] = if platform.is_mac() {
        &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
    } else {
        &["/home/linuxbrew/.linuxbrew/bin/brew"]
    };

    candidates
        .iter()
        .map(PathBuf::from)
        .find(|candidate| candidate.is_file())
}

/// Activate a verified local Homebrew installation without evaluating or
/// partially parsing the shell program emitted by `brew shellenv`.
fn activate_homebrew(brew: &Path) -> Result<()> {
    let brew_program = path_arg(brew)?;
    let prefix = PathBuf::from(command::run(brew_program, &["--prefix"])?);
    if !prefix.is_absolute() {
        bail!(
            "Homebrew returned a non-absolute prefix from {}: {}",
            brew.display(),
            prefix.display()
        );
    }

    let path = homebrew_path(&prefix, std::env::var_os("PATH").as_deref())?;
    // SAFETY: installer orchestration is single-threaded and updates the
    // process environment before spawning any subsequent tool installers.
    unsafe {
        std::env::set_var("HOMEBREW_PREFIX", &prefix);
        std::env::set_var("PATH", path);
    }

    if !has_brew() {
        bail!(
            "Homebrew executable {} is still unavailable after activating prefix {}",
            brew.display(),
            prefix.display()
        );
    }

    Ok(())
}

fn homebrew_path(prefix: &Path, current: Option<&OsStr>) -> Result<OsString> {
    let mut entries = vec![prefix.join("bin"), prefix.join("sbin")];
    if let Some(current) = current {
        for entry in std::env::split_paths(current) {
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
    }

    std::env::join_paths(entries).context("constructing PATH for Homebrew activation")
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
        ensure_brew(platform).context("Homebrew is required for package installation on macOS")?;
        return brew_install(package);
    }

    // A single-tool install must establish the same preferred package-manager
    // prerequisite that `install all` establishes in its base phase. On Linux,
    // a failed Homebrew bootstrap records the failure and falls back to the
    // distro-native manager below.
    if is_brew_applicable(platform) && !is_brew_failed() && !has_brew() {
        let _ = ensure_brew(platform);
    }

    // Linux/WSL: route based on distro
    match platform.distro() {
        Some(Distro::Debian | Distro::Ubuntu) => {
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
    bail!("Fedora/RHEL support not yet implemented. Would install: {package}")
}

/// Install a package via pacman (Arch/Manjaro).
/// Stub — not yet implemented.
pub fn pacman_install(package: &str) -> Result<()> {
    bail!("Arch Linux support not yet implemented. Would install: {package}")
}

/// Install a package via apk (Alpine).
/// Stub — not yet implemented.
pub fn apk_install(package: &str) -> Result<()> {
    bail!("Alpine Linux support not yet implemented. Would install: {package}")
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

/// Download a GPG key, verify its primary-key fingerprints, and install it for apt.
///
/// Requiring the complete expected primary-key set prevents a compromised download
/// location from silently replacing the repository key or appending another trusted
/// primary key to the installed keyring.
pub fn apt_add_gpg_key(
    url: &str,
    keyring_path: &str,
    expected_primary_fingerprints: &[&str],
) -> Result<()> {
    if expected_primary_fingerprints.is_empty() {
        bail!("at least one expected GPG primary-key fingerprint is required")
    }

    let temp_dir = tempfile::tempdir().context("creating temporary directory for apt key")?;
    let key_path = temp_dir.path().join("repo-key");

    if !command::exists("gpg") {
        privilege::run_privileged("apt-get", &["update", "-qq"])?;
        privilege::run_privileged("apt-get", &["install", "-y", "-qq", "gnupg"])?;
    }

    if url.ends_with(".gpg") {
        download::download_file(url, &key_path)?;
        verify_gpg_primary_fingerprints(&key_path, expected_primary_fingerprints)?;
        install_apt_file(&key_path, keyring_path)
    } else {
        let armored_path = temp_dir.path().join("repo-key.asc");
        let dearmored_path = temp_dir.path().join("repo-key.gpg");
        download::download_file(url, &armored_path)?;
        verify_gpg_primary_fingerprints(&armored_path, expected_primary_fingerprints)?;

        command::run_visible(
            "gpg",
            &[
                "--dearmor",
                "-o",
                path_arg(&dearmored_path)?,
                path_arg(&armored_path)?,
            ],
        )?;

        install_apt_file(&dearmored_path, keyring_path)
    }
}

fn verify_gpg_primary_fingerprints(
    key_path: &std::path::Path,
    expected_primary_fingerprints: &[&str],
) -> Result<()> {
    let output = command::run(
        "gpg",
        &[
            "--batch",
            "--show-keys",
            "--with-colons",
            path_arg(key_path)?,
        ],
    )
    .with_context(|| format!("inspecting downloaded GPG key {}", key_path.display()))?;

    let actual = primary_fingerprints_from_gpg_colons(&output)?;
    let expected = expected_primary_fingerprints
        .iter()
        .map(|fingerprint| normalize_fingerprint(fingerprint))
        .collect::<Result<BTreeSet<_>>>()?;

    if actual != expected {
        bail!(
            "downloaded GPG primary-key fingerprint mismatch for {}:\n  expected: {}\n  actual:   {}",
            key_path.display(),
            expected.iter().cloned().collect::<Vec<_>>().join(", "),
            actual.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    }

    Ok(())
}

fn primary_fingerprints_from_gpg_colons(output: &str) -> Result<BTreeSet<String>> {
    let mut fingerprints = BTreeSet::new();
    let mut awaiting_primary_fingerprint = false;

    for line in output.lines() {
        let fields = line.split(':').collect::<Vec<_>>();
        match fields.first().copied() {
            Some("pub") => awaiting_primary_fingerprint = true,
            Some("sub") => awaiting_primary_fingerprint = false,
            Some("fpr") if awaiting_primary_fingerprint => {
                let fingerprint = fields.get(9).copied().unwrap_or_default();
                fingerprints.insert(normalize_fingerprint(fingerprint)?);
                awaiting_primary_fingerprint = false;
            }
            _ => {}
        }
    }

    if fingerprints.is_empty() {
        bail!("downloaded file did not contain a GPG primary-key fingerprint")
    }

    Ok(fingerprints)
}

fn normalize_fingerprint(fingerprint: &str) -> Result<String> {
    let normalized = fingerprint
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();

    if normalized.len() != 40
        || !normalized
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        bail!("invalid GPG fingerprint: {fingerprint}")
    }

    Ok(normalized)
}

/// Add an apt repository source file and run apt update.
pub fn apt_add_repo(repo_line: &str, list_file: &str) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("creating temporary directory for apt repo")?;
    let source_path = temp_dir.path().join("repo.list");
    std::fs::write(&source_path, format!("{repo_line}\n"))
        .with_context(|| format!("writing {}", source_path.display()))?;

    install_apt_file(&source_path, list_file)?;
    privilege::run_privileged("apt-get", &["update"])
}

fn install_apt_file(source: &std::path::Path, destination: &str) -> Result<()> {
    if !std::path::Path::new(destination).is_absolute() {
        bail!("apt destination must be absolute: {destination}");
    }

    privilege::run_privileged(
        "install",
        &["-D", "-m", "0644", "--", path_arg(source)?, destination],
    )
}

fn path_arg(path: &std::path::Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("path is not valid UTF-8: {}", path.display()))
}

/// Returns true if on Linux and not root (needs sudo/privilege escalation
/// for native package operations).
///
/// NixOS never needs sudo for package operations (declarative model).
/// macOS does not use apt, so always returns false.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    fn ubuntu() -> Platform {
        Platform {
            os: Os::Linux(Distro::Ubuntu),
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
    fn brew_applicable_on_ubuntu() {
        assert!(is_brew_applicable(&ubuntu()));
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
        assert!(
            msg.contains("vim"),
            "error should contain package name: {msg}"
        );
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
        assert!(
            msg.contains("vim"),
            "error should contain package name: {msg}"
        );
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
        assert!(
            msg.contains("vim"),
            "error should contain package name: {msg}"
        );
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
    fn ubuntu_needs_sudo_when_not_root() {
        if !command::is_root() {
            assert!(needs_sudo_for_native_pkg(&ubuntu()));
        }
    }

    #[test]
    fn needs_sudo_for_apt_matches_native_pkg() {
        let platforms = [mac(), debian(), ubuntu(), fedora(), nixos(), arch_linux()];
        for p in &platforms {
            assert_eq!(
                needs_sudo_for_apt(p),
                needs_sudo_for_native_pkg(p),
                "mismatch for platform: {p}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Homebrew activation PATH
    // -----------------------------------------------------------------------

    #[test]
    fn homebrew_path_prepends_required_entries_without_duplicates() {
        let prefix = Path::new("/opt/homebrew");
        let current = std::env::join_paths([
            Path::new("/usr/bin"),
            Path::new("/opt/homebrew/bin"),
            Path::new("/bin"),
        ])
        .unwrap();

        let path = homebrew_path(prefix, Some(&current)).unwrap();
        let entries = std::env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(
            entries,
            vec![
                prefix.join("bin"),
                prefix.join("sbin"),
                PathBuf::from("/usr/bin"),
                PathBuf::from("/bin"),
            ]
        );
    }

    #[test]
    fn homebrew_path_works_without_an_existing_path() {
        let prefix = Path::new("/home/linuxbrew/.linuxbrew");

        let path = homebrew_path(prefix, None).unwrap();

        assert_eq!(
            std::env::split_paths(&path).collect::<Vec<_>>(),
            vec![prefix.join("bin"), prefix.join("sbin")]
        );
    }

    // -----------------------------------------------------------------------
    // apt_add_gpg_key format detection
    // -----------------------------------------------------------------------

    #[test]
    fn gpg_url_detects_binary_key() {
        assert!("https://cli.github.com/packages/githubcli-archive-keyring.gpg".ends_with(".gpg"));
    }

    #[test]
    fn gpg_url_detects_ascii_key() {
        assert!(!"https://packages.microsoft.com/keys/microsoft.asc".ends_with(".gpg"));
    }

    #[test]
    fn gpg_url_bare_needs_dearmor() {
        assert!(!"https://download.docker.com/linux/ubuntu/gpg".ends_with(".gpg"));
    }

    #[test]
    fn extracts_only_primary_fingerprints_from_gpg_colons() {
        let output = concat!(
            "pub:-:4096:1:23F3D4EA75716059:0:0::-:::scESC::::::23::0:\n",
            "fpr:::::::::2C6106201985B60E6C7AC87323F3D4EA75716059:\n",
            "sub:-:4096:1:E5FAF19590714157:0:0:::::e::::::23:\n",
            "fpr:::::::::5700BAB26C8DE75F3EE323FEE5FAF19590714157:\n",
            "pub:-:4096:1:5612B36462313325:0:0::-:::scESC::::::23::0:\n",
            "fpr:::::::::7F38BBB59D064DBCB3D84D725612B36462313325:\n",
        );

        let fingerprints = primary_fingerprints_from_gpg_colons(output).unwrap();
        assert_eq!(
            fingerprints,
            BTreeSet::from([
                "2C6106201985B60E6C7AC87323F3D4EA75716059".to_string(),
                "7F38BBB59D064DBCB3D84D725612B36462313325".to_string(),
            ])
        );
    }

    #[test]
    fn fingerprint_normalization_accepts_grouped_upper_or_lowercase_hex() {
        assert_eq!(
            normalize_fingerprint("bc52 8686 b50d 79e3 39d3 721c eb3e 94ad be12 29cf").unwrap(),
            "BC528686B50D79E339D3721CEB3E94ADBE1229CF"
        );
    }

    #[test]
    fn fingerprint_normalization_rejects_short_values() {
        assert!(normalize_fingerprint("BE1229CF").is_err());
    }
}
