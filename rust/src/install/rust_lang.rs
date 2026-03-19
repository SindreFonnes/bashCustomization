use anyhow::Result;

use crate::common::command;
use crate::common::platform::Platform;
use super::InstallConfig;

pub struct RustInstaller;

impl super::Installer for RustInstaller {
    fn name(&self) -> &str {
        "rust"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false // installs to ~/.cargo
    }

    fn is_installed(&self) -> bool {
        command::exists("rustc")
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!("  Would install Rust via rustup (curl | sh -s -- -y)");
            return Ok(());
        }

        println!("Installing Rust via rustup...");
        command::run_visible(
            "bash",
            &[
                "-c",
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
            ],
        )?;

        println!("Rust installed via rustup");
        Ok(())
    }
}
