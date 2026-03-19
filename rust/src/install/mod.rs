mod orchestrator;
pub mod tools;

use anyhow::Result;

use crate::common::platform::Platform;

// Re-export the public API
pub use orchestrator::{run_all, run_by_name, run_interactive};

/// Configuration passed to every installer.
pub struct InstallConfig {
    pub platform: Platform,
    pub dry_run: bool,
    pub verbose: bool,
    pub interactive: bool,
}

/// Outcome of a single install attempt.
pub enum InstallOutcome {
    Installed,
    Skipped(String),
    Failed(String),
}

/// Common interface for all tool installers.
pub trait Installer: Send + Sync {
    fn name(&self) -> &str;
    fn needs_sudo(&self, platform: &Platform) -> bool;
    fn is_installed(&self) -> bool;
    fn install(&self, config: &InstallConfig) -> Result<()>;
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
    fn phase(&self) -> u8 {
        delegate!(self, phase)
    }
}

// ---------------------------------------------------------------------------
// Registry — the single source of truth for which tools exist
// ---------------------------------------------------------------------------

/// All registered tools in installation order.
pub const ALL_TOOLS: &[Tool] = &[
    // Phase 0: base
    Tool::Brew(tools::brew::BrewInstaller),
    Tool::Base(tools::base::BaseInstaller),
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
        assert_eq!(ALL_TOOLS.len(), 21, "expected 21 tools (20 + base)");
    }

    #[test]
    fn tool_is_copy() {
        let t = ALL_TOOLS[0];
        let _t2 = t;
        let _t3 = t;
    }
}
