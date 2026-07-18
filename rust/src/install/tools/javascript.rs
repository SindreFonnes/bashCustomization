use anyhow::Result;

use crate::common::{self, command, download, package_manager, platform::Platform};
use crate::install::{InstallConfig, InstallationState, state_from_components};

const NVM_INSTALL_VERSION: &str = "v0.40.1";
const NVM_INSTALL_SHA256: &str = "abdb525ee9f5b48b34d8ed9fc67c6013fb0f659712e401ecd88ab989b3af8f53";
const PNPM_INSTALL_URL: &str = "https://get.pnpm.io/install.sh";
const PNPM_INSTALL_SHA256: &str =
    "ab8b2166653269b1182ae8ae03801b6c651fae56a0ca9e011d5d5d5aac0f056b";
const BUN_INSTALL_URL: &str = "https://bun.sh/install";
const BUN_INSTALL_SHA256: &str = "bab8acfb046aac8c72407bdcce903957665d655d7acaa3e11c7c4616beae68dd";

#[derive(Debug, Clone, Copy)]
pub struct JavaScriptInstaller;

impl crate::install::Installer for JavaScriptInstaller {
    fn name(&self) -> &str {
        "javascript"
    }

    fn needs_sudo(&self, _platform: &Platform) -> bool {
        // nvm installs to ~/.nvm, pnpm/bun to user dirs, yarn via npm -g
        // under nvm — none require root
        false
    }

    fn requires_brew(&self, _platform: &Platform) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        missing_javascript_components().is_empty()
    }

    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        javascript_installation_state()
    }

    fn install(&self, config: &InstallConfig) -> Result<()> {
        if config.dry_run {
            println!("  Would install nvm, then pnpm, bun, and yarn");
            return Ok(());
        }

        command::require_all(&["bash", "curl"])?;
        install_nvm()?;
        install_pnpm()?;
        install_bun()?;
        install_yarn()?;

        println!("JavaScript toolchain installed (nvm, pnpm, bun, yarn)");
        Ok(())
    }

    fn phase(&self) -> u8 {
        2 // JS tools must run after other tools, nvm first
    }
}

fn install_nvm() -> Result<()> {
    if !nvm_shell_exists() {
        println!("Installing nvm...");
        let install_url = format!(
            "https://raw.githubusercontent.com/nvm-sh/nvm/{NVM_INSTALL_VERSION}/install.sh"
        );
        download::run_verified_script(&install_url, NVM_INSTALL_SHA256, "bash", &[], &[])?;
    } else {
        println!("nvm already installed, checking Node.js...");
    }

    if !nvm_managed_command_exists("node") {
        println!("Installing latest Node.js LTS via nvm...");
        command::run_visible(
            "bash",
            &[
                "-c",
                r#"export NVM_DIR="${NVM_DIR:-$HOME/.nvm}" && [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && nvm install --lts"#,
            ],
        )?;
    }

    Ok(())
}

fn install_pnpm() -> Result<()> {
    if pnpm_exists() {
        println!("pnpm already installed, skipping...");
        return Ok(());
    }

    println!("Installing pnpm...");
    if command::exists("corepack") {
        return command::run_visible("corepack", &["enable", "pnpm"]);
    }

    download::run_verified_script(PNPM_INSTALL_URL, PNPM_INSTALL_SHA256, "sh", &[], &[])
}

fn install_bun() -> Result<()> {
    if bun_exists() {
        println!("bun already installed, skipping...");
        return Ok(());
    }

    println!("Installing bun...");
    download::run_verified_script(BUN_INSTALL_URL, BUN_INSTALL_SHA256, "bash", &[], &[])
}

fn install_yarn() -> Result<()> {
    if yarn_exists() {
        println!("yarn already installed, skipping...");
        return Ok(());
    }

    if !package_manager::is_brew_failed() && package_manager::has_brew() {
        println!("Installing yarn via brew...");
        return package_manager::brew_install("yarn");
    }

    println!("Installing yarn via npm from nvm...");
    command::run_visible(
        "bash",
        &[
            "-c",
            r#"export NVM_DIR="${NVM_DIR:-$HOME/.nvm}" && [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh" && npm install -g yarn"#,
        ],
    )?;

    Ok(())
}

fn nvm_dir() -> Option<std::path::PathBuf> {
    std::env::var("NVM_DIR")
        .ok()
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| common::home_dir().ok().map(|home| home.join(".nvm")))
}

fn nvm_shell_exists() -> bool {
    nvm_dir()
        .map(|directory| directory.join("nvm.sh").is_file())
        .unwrap_or(false)
}

fn nvm_managed_command_exists(name: &str) -> bool {
    let Some(versions_dir) = nvm_dir().map(|directory| directory.join("versions/node")) else {
        return false;
    };

    command_exists_in_nvm_versions(&versions_dir, name)
}

fn command_exists_in_nvm_versions(versions_dir: &std::path::Path, name: &str) -> bool {
    let Ok(versions) = std::fs::read_dir(versions_dir) else {
        return false;
    };

    versions
        .flatten()
        .any(|version| version.path().join("bin").join(name).is_file())
}

fn user_file_exists(relative_paths: &[&str]) -> bool {
    let Ok(home) = common::home_dir() else {
        return false;
    };
    relative_paths.iter().any(|path| home.join(path).is_file())
}

fn pnpm_exists() -> bool {
    command::exists("pnpm")
        || std::env::var("PNPM_HOME")
            .ok()
            .map(|home| std::path::Path::new(&home).join("pnpm").is_file())
            .unwrap_or(false)
        || user_file_exists(&[".local/share/pnpm/pnpm", ".local/bin/pnpm"])
}

fn bun_exists() -> bool {
    command::exists("bun")
        || std::env::var("BUN_INSTALL")
            .ok()
            .map(|home| std::path::Path::new(&home).join("bin/bun").is_file())
            .unwrap_or(false)
        || user_file_exists(&[".bun/bin/bun"])
}

fn yarn_exists() -> bool {
    command::exists("yarn")
        || nvm_managed_command_exists("yarn")
        || user_file_exists(&[".yarn/bin/yarn", ".local/bin/yarn"])
}

fn missing_javascript_components() -> Vec<&'static str> {
    [
        ("nvm", nvm_shell_exists()),
        ("node", nvm_managed_command_exists("node")),
        ("pnpm", pnpm_exists()),
        ("bun", bun_exists()),
        ("yarn", yarn_exists()),
    ]
    .into_iter()
    .filter_map(|(name, exists)| (!exists).then_some(name))
    .collect()
}

fn javascript_installation_state() -> InstallationState {
    classify_javascript_components([
        nvm_shell_exists(),
        nvm_managed_command_exists("node"),
        pnpm_exists(),
        bun_exists(),
        yarn_exists(),
    ])
}

fn classify_javascript_components(present: [bool; 5]) -> InstallationState {
    state_from_components(&[
        ("nvm", present[0]),
        ("node", present[1]),
        ("pnpm", present[2]),
        ("bun", present[3]),
        ("yarn", present[4]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};
    use crate::install::Installer;

    #[test]
    fn needs_sudo_always_false() {
        let platforms = [
            Platform {
                os: Os::Linux(Distro::Debian),
                arch: Arch::X86_64,
            },
            Platform {
                os: Os::Linux(Distro::NixOs),
                arch: Arch::X86_64,
            },
            Platform {
                os: Os::MacOs,
                arch: Arch::Aarch64,
            },
            Platform {
                os: Os::Linux(Distro::Fedora),
                arch: Arch::X86_64,
            },
        ];
        for p in &platforms {
            assert!(
                !JavaScriptInstaller.needs_sudo(p),
                "needs_sudo should be false for {p}"
            );
        }
    }

    #[test]
    fn partial_toolchain_is_incomplete_and_names_missing_components() {
        let state = classify_javascript_components([true, true, false, true, false]);
        assert_eq!(
            state,
            InstallationState::Incomplete("missing pnpm, yarn".to_string())
        );
    }

    #[test]
    fn finds_node_only_inside_an_nvm_version() {
        let dir = tempfile::tempdir().unwrap();
        let versions_dir = dir.path().join("versions/node");
        let node = versions_dir.join("v22.0.0/bin/node");
        std::fs::create_dir_all(node.parent().unwrap()).unwrap();

        assert!(!command_exists_in_nvm_versions(&versions_dir, "node"));
        std::fs::write(&node, "test executable").unwrap();
        assert!(command_exists_in_nvm_versions(&versions_dir, "node"));
    }
}
