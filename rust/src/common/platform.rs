use std::fmt;

use anyhow::{Result, bail};

/// Linux distribution family, detected from `/etc/os-release`.
///
/// `Ubuntu` is tracked separately from `Debian` because some third-party apt
/// repositories use different repo URLs and codenames for each (e.g. Docker,
/// Azure CLI, .NET SDK, Terraform). For "is this apt-based?" checks, use
/// `Platform::is_debian()` which returns true for both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Distro {
    Debian,
    Ubuntu,
    Fedora,
    Arch,
    Alpine,
    NixOs,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux(Distro),
    Wsl(Distro),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

/// Parse the `ID` and `ID_LIKE` fields from `/etc/os-release` content and
/// return the corresponding `Distro`.
///
/// Matching strategy:
/// 1. Check `ID` for an exact known distro name.
/// 2. If `ID` is unknown, check each token in `ID_LIKE` (space-separated).
/// 3. If nothing matches, return `Distro::Unknown` with the original `ID` value.
pub fn parse_distro(os_release: &str) -> Distro {
    let mut id = None;
    let mut id_like = None;

    for line in os_release.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("ID=") {
            id = Some(unquote(value));
        } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
            id_like = Some(unquote(value));
        }
    }

    let id_str = match &id {
        Some(s) => s.as_str(),
        None => return Distro::Unknown(String::new()),
    };

    // Try matching on ID first
    if let Some(d) = match_distro_token(id_str) {
        return d;
    }

    // Try matching on ID_LIKE tokens
    if let Some(like) = &id_like {
        for token in like.split_whitespace() {
            if let Some(d) = match_distro_token(token) {
                return d;
            }
        }
    }

    Distro::Unknown(id_str.to_string())
}

/// Match a single lowercase token to a known distro.
fn match_distro_token(token: &str) -> Option<Distro> {
    match token {
        "debian" => Some(Distro::Debian),
        "ubuntu" => Some(Distro::Ubuntu),
        "fedora" | "rhel" | "centos" => Some(Distro::Fedora),
        "arch" | "manjaro" => Some(Distro::Arch),
        "alpine" => Some(Distro::Alpine),
        "nixos" => Some(Distro::NixOs),
        _ => None,
    }
}

/// Remove surrounding double quotes from a value, if present.
fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Read `/etc/os-release` and parse the distro. Returns `Distro::Unknown("")`
/// if the file is missing or unreadable (e.g. on macOS).
fn detect_distro() -> Distro {
    match std::fs::read_to_string("/etc/os-release") {
        Ok(content) => parse_distro(&content),
        Err(_) => Distro::Unknown(String::new()),
    }
}

/// Read `VERSION_CODENAME` from `/etc/os-release` (e.g. "jammy", "bookworm").
/// Returns `None` on macOS or if the field is missing.
pub fn get_apt_codename() -> Option<String> {
    let content = std::fs::read_to_string("/etc/os-release").ok()?;
    parse_os_release_field(&content, "VERSION_CODENAME")
}

/// Parse a specific field from os-release content by key name.
/// Handles both quoted and unquoted values.
pub fn parse_os_release_field(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for line in content.lines() {
        if let Some(val) = line.strip_prefix(&prefix) {
            let val = val.trim_matches('"').trim_matches('\'');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

impl Platform {
    pub fn detect() -> Result<Self> {
        let arch = match std::env::consts::ARCH {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            other => bail!(
                "Unsupported architecture: {other}. Supported: x86_64, aarch64"
            ),
        };

        let os = match std::env::consts::OS {
            "macos" => Os::MacOs,
            "linux" => {
                let distro = detect_distro();
                if is_wsl() {
                    Os::Wsl(distro)
                } else {
                    Os::Linux(distro)
                }
            }
            other => bail!(
                "Unsupported OS: {other}. Supported: macOS, Linux, WSL"
            ),
        };

        Ok(Platform { os, arch })
    }

    pub fn is_mac(&self) -> bool {
        self.os == Os::MacOs
    }

    /// Returns true for both native Linux and WSL.
    pub fn is_linux(&self) -> bool {
        matches!(self.os, Os::Linux(_) | Os::Wsl(_))
    }

    pub fn is_wsl(&self) -> bool {
        matches!(self.os, Os::Wsl(_))
    }

    /// Returns the distro if running on Linux or WSL, `None` on macOS.
    pub fn distro(&self) -> Option<&Distro> {
        match &self.os {
            Os::Linux(d) | Os::Wsl(d) => Some(d),
            Os::MacOs => None,
        }
    }

    /// Returns true if the distro is Debian-family (Debian or Ubuntu).
    /// Use this for apt/dpkg operations that work identically on both.
    pub fn is_debian(&self) -> bool {
        matches!(self.distro(), Some(Distro::Debian | Distro::Ubuntu))
    }

    /// Returns true if the distro is specifically Ubuntu (not plain Debian).
    /// Use this for Ubuntu-specific operations like `add-apt-repository universe`.
    pub fn is_ubuntu(&self) -> bool {
        self.distro() == Some(&Distro::Ubuntu)
    }

    /// Returns true if the distro is Fedora (or a derivative like Rocky, CentOS).
    pub fn is_fedora(&self) -> bool {
        self.distro() == Some(&Distro::Fedora)
    }

    /// Returns true if the distro is Arch (or a derivative like Manjaro).
    pub fn is_arch(&self) -> bool {
        self.distro() == Some(&Distro::Arch)
    }

    /// Returns true if the distro is Alpine.
    pub fn is_alpine(&self) -> bool {
        self.distro() == Some(&Distro::Alpine)
    }

    /// Returns true if the distro is NixOS.
    pub fn is_nixos(&self) -> bool {
        self.distro() == Some(&Distro::NixOs)
    }

    /// Go-style OS string for download URLs.
    pub fn go_os(&self) -> &'static str {
        match self.os {
            Os::MacOs => "darwin",
            Os::Linux(_) | Os::Wsl(_) => "linux",
        }
    }

    /// Go-style architecture string for download URLs.
    pub fn go_arch(&self) -> &'static str {
        match self.arch {
            Arch::X86_64 => "amd64",
            Arch::Aarch64 => "arm64",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let os_str = match &self.os {
            Os::MacOs => "macOS".to_string(),
            Os::Linux(distro) => format!("Linux ({distro:?})"),
            Os::Wsl(distro) => format!("WSL ({distro:?})"),
        };
        let arch_str = match self.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        };
        write!(f, "{os_str} ({arch_str})")
    }
}

fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("wsl"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_distro tests (TDD — written before implementation)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_ubuntu() {
        let content = "\
NAME=\"Ubuntu\"
VERSION=\"22.04.3 LTS (Jammy Jellyfish)\"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME=\"Ubuntu 22.04.3 LTS\"
";
        assert_eq!(parse_distro(content), Distro::Ubuntu);
    }

    #[test]
    fn parse_debian() {
        let content = "\
PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"
NAME=\"Debian GNU/Linux\"
ID=debian
VERSION_ID=\"12\"
";
        assert_eq!(parse_distro(content), Distro::Debian);
    }

    #[test]
    fn parse_fedora() {
        let content = "\
NAME=\"Fedora Linux\"
VERSION=\"39 (Workstation Edition)\"
ID=fedora
PRETTY_NAME=\"Fedora Linux 39 (Workstation Edition)\"
";
        assert_eq!(parse_distro(content), Distro::Fedora);
    }

    #[test]
    fn parse_rocky_linux() {
        let content = "\
NAME=\"Rocky Linux\"
VERSION=\"9.3 (Blue Onyx)\"
ID=\"rocky\"
ID_LIKE=\"rhel centos fedora\"
PRETTY_NAME=\"Rocky Linux 9.3 (Blue Onyx)\"
";
        assert_eq!(parse_distro(content), Distro::Fedora);
    }

    #[test]
    fn parse_arch() {
        let content = "\
NAME=\"Arch Linux\"
PRETTY_NAME=\"Arch Linux\"
ID=arch
BUILD_ID=rolling
";
        assert_eq!(parse_distro(content), Distro::Arch);
    }

    #[test]
    fn parse_manjaro() {
        let content = "\
NAME=\"Manjaro Linux\"
ID=manjaro
ID_LIKE=arch
PRETTY_NAME=\"Manjaro Linux\"
";
        assert_eq!(parse_distro(content), Distro::Arch);
    }

    #[test]
    fn parse_alpine() {
        let content = "\
NAME=\"Alpine Linux\"
ID=alpine
VERSION_ID=3.19.0
PRETTY_NAME=\"Alpine Linux v3.19\"
";
        assert_eq!(parse_distro(content), Distro::Alpine);
    }

    #[test]
    fn parse_nixos() {
        let content = "\
NAME=NixOS
ID=nixos
VERSION=\"23.11 (Tapir)\"
PRETTY_NAME=\"NixOS 23.11 (Tapir)\"
";
        assert_eq!(parse_distro(content), Distro::NixOs);
    }

    #[test]
    fn parse_unknown_distro() {
        let content = "\
NAME=\"Gentoo\"
ID=gentoo
PRETTY_NAME=\"Gentoo Linux\"
";
        assert_eq!(parse_distro(content), Distro::Unknown("gentoo".to_string()));
    }

    #[test]
    fn parse_empty_content() {
        assert_eq!(parse_distro(""), Distro::Unknown(String::new()));
    }

    #[test]
    fn parse_id_like_fallback() {
        // A hypothetical derivative where ID is unknown but ID_LIKE points to debian
        let content = "\
NAME=\"CustomOS\"
ID=customos
ID_LIKE=debian
";
        assert_eq!(parse_distro(content), Distro::Debian);
    }

    #[test]
    fn parse_id_like_multiple_tokens() {
        // ID_LIKE with multiple tokens; first matching token wins
        let content = "\
NAME=\"CentOS Stream\"
ID=\"centos\"
ID_LIKE=\"rhel fedora\"
";
        // centos matches Fedora directly via match_distro_token
        assert_eq!(parse_distro(content), Distro::Fedora);
    }

    // -----------------------------------------------------------------------
    // Platform-level tests (updated for new Os shape)
    // -----------------------------------------------------------------------

    #[test]
    fn detect_returns_valid_platform() {
        let platform = Platform::detect().expect("should detect current platform");
        assert!(
            matches!(platform.os, Os::MacOs | Os::Linux(_) | Os::Wsl(_)),
            "unexpected OS: {:?}",
            platform.os
        );
        assert!(
            matches!(platform.arch, Arch::X86_64 | Arch::Aarch64),
            "unexpected arch: {:?}",
            platform.arch
        );
    }

    #[test]
    fn go_os_strings() {
        let mac = Platform { os: Os::MacOs, arch: Arch::X86_64 };
        assert_eq!(mac.go_os(), "darwin");

        let linux = Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 };
        assert_eq!(linux.go_os(), "linux");

        let wsl = Platform { os: Os::Wsl(Distro::Debian), arch: Arch::X86_64 };
        assert_eq!(wsl.go_os(), "linux");
    }

    #[test]
    fn go_arch_strings() {
        let x86 = Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 };
        assert_eq!(x86.go_arch(), "amd64");

        let arm = Platform { os: Os::Linux(Distro::Debian), arch: Arch::Aarch64 };
        assert_eq!(arm.go_arch(), "arm64");
    }

    #[test]
    fn is_linux_includes_wsl() {
        let wsl = Platform { os: Os::Wsl(Distro::Debian), arch: Arch::X86_64 };
        assert!(wsl.is_linux());
        assert!(wsl.is_wsl());

        let linux = Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 };
        assert!(linux.is_linux());
        assert!(!linux.is_wsl());
    }

    #[test]
    fn distro_accessors() {
        let mac = Platform { os: Os::MacOs, arch: Arch::X86_64 };
        assert_eq!(mac.distro(), None);
        assert!(!mac.is_debian());

        let debian = Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 };
        assert_eq!(debian.distro(), Some(&Distro::Debian));
        assert!(debian.is_debian());
        assert!(!debian.is_ubuntu());
        assert!(!debian.is_fedora());

        let ubuntu = Platform { os: Os::Linux(Distro::Ubuntu), arch: Arch::X86_64 };
        assert_eq!(ubuntu.distro(), Some(&Distro::Ubuntu));
        assert!(ubuntu.is_debian(), "Ubuntu should be considered debian-family");
        assert!(ubuntu.is_ubuntu());
        assert!(!ubuntu.is_fedora());

        let fedora = Platform { os: Os::Linux(Distro::Fedora), arch: Arch::X86_64 };
        assert!(fedora.is_fedora());
        assert!(!fedora.is_debian());

        let arch = Platform { os: Os::Linux(Distro::Arch), arch: Arch::X86_64 };
        assert!(arch.is_arch());

        let alpine = Platform { os: Os::Linux(Distro::Alpine), arch: Arch::X86_64 };
        assert!(alpine.is_alpine());

        let nixos = Platform { os: Os::Linux(Distro::NixOs), arch: Arch::X86_64 };
        assert!(nixos.is_nixos());
    }

    #[test]
    fn display_includes_distro() {
        let linux = Platform { os: Os::Linux(Distro::Debian), arch: Arch::X86_64 };
        let display = format!("{linux}");
        assert!(display.contains("Linux"), "display should contain Linux: {display}");
        assert!(display.contains("Debian"), "display should contain Debian: {display}");
        assert!(display.contains("x86_64"), "display should contain x86_64: {display}");

        let mac = Platform { os: Os::MacOs, arch: Arch::Aarch64 };
        let display = format!("{mac}");
        assert!(display.contains("macOS"), "display should contain macOS: {display}");
        assert!(display.contains("aarch64"), "display should contain aarch64: {display}");
    }

    // -----------------------------------------------------------------------
    // parse_os_release_field tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_os_release_field_extracts_codename() {
        let content = "ID=ubuntu\nVERSION_CODENAME=jammy\nVERSION_ID=\"22.04\"\n";
        assert_eq!(
            parse_os_release_field(content, "VERSION_CODENAME"),
            Some("jammy".to_string())
        );
    }

    #[test]
    fn parse_os_release_field_handles_quoted() {
        let content = "ID=\"debian\"\nVERSION_CODENAME=\"bookworm\"\n";
        assert_eq!(
            parse_os_release_field(content, "VERSION_CODENAME"),
            Some("bookworm".to_string())
        );
    }

    #[test]
    fn parse_os_release_field_returns_none_when_missing() {
        let content = "ID=alpine\nVERSION_ID=3.19.0\n";
        assert_eq!(parse_os_release_field(content, "VERSION_CODENAME"), None);
    }

    #[test]
    fn parse_os_release_field_returns_none_when_empty() {
        let content = "VERSION_CODENAME=\n";
        assert_eq!(parse_os_release_field(content, "VERSION_CODENAME"), None);
    }

    #[test]
    fn ubuntu_id_like_fallback_stays_debian() {
        // A derivative with ID_LIKE=ubuntu should fall through to Ubuntu
        let content = "\
NAME=\"Linux Mint\"
ID=linuxmint
ID_LIKE=\"ubuntu debian\"
";
        // ID_LIKE token "ubuntu" matches Distro::Ubuntu
        assert_eq!(parse_distro(content), Distro::Ubuntu);
    }
}
