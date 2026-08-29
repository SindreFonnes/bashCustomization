mod orchestrator;
pub mod tools;

use anyhow::Result;

use crate::common::platform::Platform;

// Re-export the public API
pub use orchestrator::{run_by_name, run_interactive};

/// Configuration passed to every installer.
pub struct InstallConfig {
    pub platform: Platform,
    pub dry_run: bool,
}

/// Whether an installer's declared user-facing outcome is currently present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallationState {
    Missing,
    Incomplete(String),
    Complete,
}

/// Classify a composite installer from its named required components.
pub(crate) fn state_from_components(components: &[(&str, bool)]) -> InstallationState {
    let missing: Vec<&str> = components
        .iter()
        .filter_map(|(name, present)| (!present).then_some(*name))
        .collect();

    if missing.is_empty() {
        InstallationState::Complete
    } else if missing.len() == components.len() {
        InstallationState::Missing
    } else {
        InstallationState::Incomplete(format!("missing {}", missing.join(", ")))
    }
}

/// Outcome of a single install attempt.
pub enum InstallOutcome {
    Installed,
    Repaired(String),
    Skipped(String),
    NotApplicable(String),
    Guidance(String),
    Planned,
    Failed(String),
}

/// Common interface for all tool installers.
pub trait Installer: Send + Sync {
    fn name(&self) -> &str;
    fn needs_sudo(&self, platform: &Platform) -> bool;
    fn is_installed(&self) -> bool;
    fn install(&self, config: &InstallConfig) -> Result<()>;
    fn installation_state(&self, _platform: &Platform) -> InstallationState {
        if self.is_installed() {
            InstallationState::Complete
        } else {
            InstallationState::Missing
        }
    }
    fn verify_installation(&self, platform: &Platform) -> Result<()> {
        match self.installation_state(platform) {
            InstallationState::Complete => Ok(()),
            InstallationState::Missing => {
                anyhow::bail!("required installation postcondition is still missing")
            }
            InstallationState::Incomplete(reason) => {
                anyhow::bail!("installation remains incomplete: {reason}")
            }
        }
    }
    fn requires_brew(&self, platform: &Platform) -> bool {
        // Most macOS installers use Homebrew as their only supported path.
        // User-local installers and Homebrew itself override this default.
        platform.is_mac()
    }
    fn is_applicable(&self, _platform: &Platform) -> bool {
        true
    }
    fn include_in_all(&self, platform: &Platform) -> bool {
        self.is_applicable(platform)
    }
    fn phase(&self) -> u8 {
        1
    }
}

// ---------------------------------------------------------------------------
// Tool enum — closed set of all known installers
// ---------------------------------------------------------------------------

/// Every variant wraps a zero-sized unit struct, so Tool is Copy with no heap
/// allocation. Adding a new installer without a match arm is a compile error.
#[derive(Debug, Clone, Copy)]
pub enum Tool {
    Doas(tools::doas::DoasInstaller),
    Brew(tools::brew::BrewInstaller),
    Base(tools::base::BaseInstaller),
    Go(tools::go::GoInstaller),
    Rust(tools::rust_lang::RustInstaller),
    Docker(tools::docker::DockerInstaller),
    Azure(tools::azure::AzureInstaller),
    Dotnet(tools::dotnet::DotnetInstaller),
    Neovim(tools::neovim::NeovimInstaller),
    Obsidian(tools::obsidian::ObsidianInstaller),
    Java(tools::java::JavaInstaller),
    Github(tools::github::GithubCliInstaller),
    Terraform(tools::terraform::TerraformInstaller),
    Postgres(tools::postgres::PostgresInstaller),
    Kubectl(tools::kubectl::KubectlInstaller),
    Ripgrep(tools::ripgrep::RipgrepInstaller),
    Bat(tools::bat::BatInstaller),
    Fd(tools::fd::FdInstaller),
    Eza(tools::eza::EzaInstaller),
    Shellcheck(tools::shellcheck::ShellcheckInstaller),
    NerdFont(tools::nerd_font::NerdFontInstaller),
    JavaScript(tools::javascript::JavaScriptInstaller),
}

/// Delegate every Installer method to the inner struct.
macro_rules! delegate {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            Tool::Doas(i)       => i.$method($($arg),*),
            Tool::Brew(i)       => i.$method($($arg),*),
            Tool::Base(i)       => i.$method($($arg),*),
            Tool::Go(i)         => i.$method($($arg),*),
            Tool::Rust(i)       => i.$method($($arg),*),
            Tool::Docker(i)     => i.$method($($arg),*),
            Tool::Azure(i)      => i.$method($($arg),*),
            Tool::Dotnet(i)     => i.$method($($arg),*),
            Tool::Neovim(i)     => i.$method($($arg),*),
            Tool::Obsidian(i)   => i.$method($($arg),*),
            Tool::Java(i)       => i.$method($($arg),*),
            Tool::Github(i)     => i.$method($($arg),*),
            Tool::Terraform(i)  => i.$method($($arg),*),
            Tool::Postgres(i)   => i.$method($($arg),*),
            Tool::Kubectl(i)    => i.$method($($arg),*),
            Tool::Ripgrep(i)    => i.$method($($arg),*),
            Tool::Bat(i)        => i.$method($($arg),*),
            Tool::Fd(i)         => i.$method($($arg),*),
            Tool::Eza(i)        => i.$method($($arg),*),
            Tool::Shellcheck(i) => i.$method($($arg),*),
            Tool::NerdFont(i)   => i.$method($($arg),*),
            Tool::JavaScript(i) => i.$method($($arg),*),
        }
    };
}

impl Installer for Tool {
    fn name(&self) -> &str {
        delegate!(self, name)
    }
    fn needs_sudo(&self, platform: &Platform) -> bool {
        delegate!(self, needs_sudo, platform)
    }
    fn is_installed(&self) -> bool {
        delegate!(self, is_installed)
    }
    fn install(&self, config: &InstallConfig) -> Result<()> {
        delegate!(self, install, config)
    }
    fn installation_state(&self, platform: &Platform) -> InstallationState {
        delegate!(self, installation_state, platform)
    }
    fn verify_installation(&self, platform: &Platform) -> Result<()> {
        delegate!(self, verify_installation, platform)
    }
    fn requires_brew(&self, platform: &Platform) -> bool {
        delegate!(self, requires_brew, platform)
    }
    fn is_applicable(&self, platform: &Platform) -> bool {
        delegate!(self, is_applicable, platform)
    }
    fn include_in_all(&self, platform: &Platform) -> bool {
        delegate!(self, include_in_all, platform)
    }
    fn phase(&self) -> u8 {
        delegate!(self, phase)
    }
}

// ---------------------------------------------------------------------------
// Registry — the single source of truth for which tools exist
// ---------------------------------------------------------------------------

/// All registered tools in installation order.
pub const ALL_TOOLS: &[Tool] = &[
    // Phase 0: bootstrap native prerequisites before Homebrew. On macOS the
    // Base installer establishes Brew through its prerequisite hook.
    Tool::Doas(tools::doas::DoasInstaller),
    Tool::Base(tools::base::BaseInstaller),
    Tool::Brew(tools::brew::BrewInstaller),
    // Phase 1: parallel tools
    Tool::Go(tools::go::GoInstaller),
    Tool::Rust(tools::rust_lang::RustInstaller),
    Tool::Docker(tools::docker::DockerInstaller),
    Tool::Azure(tools::azure::AzureInstaller),
    Tool::Dotnet(tools::dotnet::DotnetInstaller),
    Tool::Neovim(tools::neovim::NeovimInstaller),
    Tool::Obsidian(tools::obsidian::ObsidianInstaller),
    Tool::Java(tools::java::JavaInstaller),
    Tool::Github(tools::github::GithubCliInstaller),
    Tool::Terraform(tools::terraform::TerraformInstaller),
    Tool::Postgres(tools::postgres::PostgresInstaller),
    Tool::Kubectl(tools::kubectl::KubectlInstaller),
    Tool::Ripgrep(tools::ripgrep::RipgrepInstaller),
    Tool::Bat(tools::bat::BatInstaller),
    Tool::Fd(tools::fd::FdInstaller),
    Tool::Eza(tools::eza::EzaInstaller),
    Tool::Shellcheck(tools::shellcheck::ShellcheckInstaller),
    Tool::NerdFont(tools::nerd_font::NerdFontInstaller),
    // Phase 2: JS sequential
    Tool::JavaScript(tools::javascript::JavaScriptInstaller),
];

/// Return list of all available tool names.
pub fn available_tool_names() -> Vec<&'static str> {
    ALL_TOOLS.iter().map(|t| t.name()).collect()
}

/// Find a tool by name.
pub fn find_tool(name: &str) -> Option<Tool> {
    ALL_TOOLS.iter().copied().find(|t| t.name() == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os};

    #[test]
    fn find_tool_known() {
        assert!(find_tool("go").is_some());
        assert!(find_tool("rust").is_some());
        assert!(find_tool("kubectl").is_some());
    }

    #[test]
    fn find_tool_unknown() {
        assert!(find_tool("nonexistent").is_none());
    }

    #[test]
    fn all_tools_count() {
        assert_eq!(ALL_TOOLS.len(), 22, "expected 22 tools (21 + doas)");
    }

    #[test]
    fn tool_is_copy() {
        let t = ALL_TOOLS[0];
        let _t2 = t;
        let _t3 = t;
    }

    #[test]
    fn composite_state_distinguishes_missing_incomplete_and_complete() {
        assert_eq!(
            state_from_components(&[("one", false), ("two", false)]),
            InstallationState::Missing
        );
        assert_eq!(
            state_from_components(&[("one", true), ("two", false)]),
            InstallationState::Incomplete("missing two".to_string())
        );
        assert_eq!(
            state_from_components(&[("one", true), ("two", true)]),
            InstallationState::Complete
        );
    }

    #[test]
    fn formula_backed_tools_prefer_brew_on_ubuntu() {
        let ubuntu = Platform {
            os: Os::Linux(Distro::Ubuntu),
            arch: Arch::X86_64,
        };

        for name in [
            "azure",
            "bat",
            "dotnet",
            "eza",
            "fd",
            "github",
            "go",
            "java",
            "javascript",
            "kubectl",
            "neovim",
            "postgres",
            "ripgrep",
            "rust",
            "shellcheck",
            "terraform",
        ] {
            let tool = find_tool(name).unwrap_or_else(|| panic!("missing tool {name}"));
            assert!(tool.requires_brew(&ubuntu), "{name} should prefer Brew");
        }

        for name in ["base", "docker", "nerd-font", "obsidian"] {
            let tool = find_tool(name).unwrap_or_else(|| panic!("missing tool {name}"));
            assert!(
                !tool.requires_brew(&ubuntu),
                "{name} should keep its Linux-specific path"
            );
        }
    }

    #[test]
    fn linux_base_precedes_brew_in_phase_zero() {
        let base = ALL_TOOLS
            .iter()
            .position(|tool| tool.name() == "base")
            .unwrap();
        let brew = ALL_TOOLS
            .iter()
            .position(|tool| tool.name() == "brew")
            .unwrap();
        assert!(base < brew);
    }
}
