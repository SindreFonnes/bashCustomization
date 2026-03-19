use std::fmt;

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
    Wsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
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
                if is_wsl() {
                    Os::Wsl
                } else {
                    Os::Linux
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
        matches!(self.os, Os::Linux | Os::Wsl)
    }

    pub fn is_wsl(&self) -> bool {
        self.os == Os::Wsl
    }

    /// Go-style OS string for download URLs.
    pub fn go_os(&self) -> &'static str {
        match self.os {
            Os::MacOs => "darwin",
            Os::Linux | Os::Wsl => "linux",
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
        let os_str = match self.os {
            Os::MacOs => "macOS",
            Os::Linux => "Linux",
            Os::Wsl => "WSL",
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

    #[test]
    fn detect_returns_valid_platform() {
        let platform = Platform::detect().expect("should detect current platform");
        // We're running on a real machine, so it should be one of the supported combos
        assert!(
            matches!(platform.os, Os::MacOs | Os::Linux | Os::Wsl),
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

        let linux = Platform { os: Os::Linux, arch: Arch::X86_64 };
        assert_eq!(linux.go_os(), "linux");

        let wsl = Platform { os: Os::Wsl, arch: Arch::X86_64 };
        assert_eq!(wsl.go_os(), "linux");
    }

    #[test]
    fn go_arch_strings() {
        let x86 = Platform { os: Os::Linux, arch: Arch::X86_64 };
        assert_eq!(x86.go_arch(), "amd64");

        let arm = Platform { os: Os::Linux, arch: Arch::Aarch64 };
        assert_eq!(arm.go_arch(), "arm64");
    }

    #[test]
    fn is_linux_includes_wsl() {
        let wsl = Platform { os: Os::Wsl, arch: Arch::X86_64 };
        assert!(wsl.is_linux());
        assert!(wsl.is_wsl());

        let linux = Platform { os: Os::Linux, arch: Arch::X86_64 };
        assert!(linux.is_linux());
        assert!(!linux.is_wsl());
    }
}
