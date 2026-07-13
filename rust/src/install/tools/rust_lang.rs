use anyhow::Result;

use crate::common::platform::Platform;
use crate::common::{command, download};
use crate::install::{InstallConfig, InstallationState, state_from_components};

const RUSTUP_INSTALL_URL: &str = "https://sh.rustup.rs";
const RUSTUP_INSTALL_SHA256: &str =
    "6c30b75a75b28a96fd913a037c8581b580080b6ee9b8169a3c0feb1af7fe8caf";

#[derive(Debug, Clone, Copy)]
pub struct RustInstaller;

impl crate::install::Installer for RustInstaller {
    fn name(&self) -> &str {
        "rust"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        false // installs to ~/.cargo
    }

    fn requires_brew(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        missing_rust_components().is_empty()
    }

    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        rust_installation_state()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!(
                "  Would download the pinned rustup installer, verify SHA-256, and run it with -y"
            );
            return Ok(());
        }

        // NixOS: emit declarative guidance
        if config.platform.is_nixos() {
            return crate::common::package_manager::nix_guidance("rustc");
        }

        command::require_all(&["bash", "curl"])?;
        println!("Installing Rust via rustup...");
        download::run_verified_script(
            RUSTUP_INSTALL_URL,
            RUSTUP_INSTALL_SHA256,
            "sh",
            &[],
            &["-y"],
        )?;

        println!("Rust installed via rustup");
        Ok(())
    }
}

fn rust_component_exists(name: &str) -> bool {
    command::exists(name)
        || crate::common::home_dir()
            .map(|home| home.join(".cargo").join("bin").join(name).is_file())
            .unwrap_or(false)
}

fn missing_rust_components() -> Vec<&'static str> {
    ["rustc", "cargo", "rustup"]
        .into_iter()
        .filter(|name| !rust_component_exists(name))
        .collect()
}

fn rust_installation_state() -> InstallationState {
    state_from_components(&[
        ("rustc", rust_component_exists("rustc")),
        ("cargo", rust_component_exists("cargo")),
        ("rustup", rust_component_exists("rustup")),
    ])
}
