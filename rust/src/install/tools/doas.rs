use anyhow::{Result, bail};

use crate::common::{command, package_manager, platform::Platform};
use crate::install::InstallConfig;

#[derive(Debug, Clone, Copy)]
pub struct DoasInstaller;

impl crate::install::Installer for DoasInstaller {
    fn name(&self) -> &str {
        "doas"
    }

    /// doas requires root directly — there is no sudo to escalate through
    /// (chicken-and-egg: you need root to install the tool that grants root).
    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        command::exists("doas")
    }

    fn is_applicable(&self, platform: &Platform) -> bool {
        platform.is_alpine() || platform.is_debian()
    }

    fn include_in_all(&self, platform: &Platform) -> bool {
        platform.is_alpine()
            && command::is_root()
            && !crate::common::privilege::has_path_escalator()
    }

    fn requires_brew(&self, _platform: &Platform) -> bool {
        false
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        let platform = &config.platform;

        if platform.is_mac() {
            bail!("doas is not applicable on macOS (sudo is built-in)");
        }

        if config.dry_run {
            if platform.is_alpine() {
                println!("  Would install doas via apk");
            } else if platform.is_debian() {
                println!("  Would install doas via apt");
            } else {
                let distro = platform.distro();
                println!("  Would install doas (unsupported on {distro:?})");
            }
            return Ok(());
        }

        // Installing doas requires root — it cannot use sudo (which it replaces)
        if !command::is_root() {
            bail!("Installing doas requires root. Run as root: bashc install doas");
        }

        if platform.is_alpine() {
            println!("Installing doas via apk...");
            command::run_visible("apk", &["add", "doas"])?;
        } else if platform.is_debian() {
            println!("Installing doas via apt...");
            package_manager::apt_install("doas")?;
        } else {
            let distro = platform.distro();
            bail!("doas installation not yet supported on {distro:?}");
        }

        // Create a default doas configuration using the platform's normal
        // admin group. Debian/Ubuntu commonly use `sudo`; Alpine uses `wheel`.
        let admin_group = doas_admin_group(platform);
        std::fs::create_dir_all("/etc/doas.d")?;
        std::fs::write(
            "/etc/doas.d/doas.conf",
            format!("permit persist :{admin_group}\n"),
        )?;

        println!("doas installed. Add your user to the {admin_group} group if needed.");

        Ok(())
    }

    fn phase(&self) -> u8 {
        0 // Phase 0: bootstraps privilege escalation before other tools
    }
}

fn doas_admin_group(platform: &Platform) -> &'static str {
    if platform.is_debian() {
        "sudo"
    } else {
        "wheel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};

    #[test]
    fn debian_uses_sudo_group() {
        let platform = Platform {
            os: Os::Linux(Distro::Debian),
            arch: Arch::X86_64,
        };
        assert_eq!(doas_admin_group(&platform), "sudo");
    }

    #[test]
    fn alpine_uses_wheel_group() {
        let platform = Platform {
            os: Os::Linux(Distro::Alpine),
            arch: Arch::X86_64,
        };
        assert_eq!(doas_admin_group(&platform), "wheel");
    }
}
