mod azure;
mod base;
mod bat;
mod brew;
mod docker;
mod dotnet;
mod eza;
mod fd;
mod github;
mod go;
mod java;
mod javascript;
mod kubectl;
mod nerd_font;
mod neovim;
mod obsidian;
mod postgres;
mod ripgrep;
mod rust_lang;
mod shellcheck;
mod terraform;

use std::sync::Arc;

use anyhow::{Result, bail};

use crate::common::platform::Platform;

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

/// Enum of all known installers. Every variant is a zero-sized unit struct,
/// so Tool is Copy and requires no heap allocation.
#[derive(Debug, Clone, Copy)]
pub enum Tool {
    Brew(brew::BrewInstaller),
    Base(base::BaseInstaller),
    Go(go::GoInstaller),
    Rust(rust_lang::RustInstaller),
    Docker(docker::DockerInstaller),
    Azure(azure::AzureInstaller),
    Dotnet(dotnet::DotnetInstaller),
    Neovim(neovim::NeovimInstaller),
    Obsidian(obsidian::ObsidianInstaller),
    Java(java::JavaInstaller),
    Github(github::GithubCliInstaller),
    Terraform(terraform::TerraformInstaller),
    Postgres(postgres::PostgresInstaller),
    Kubectl(kubectl::KubectlInstaller),
    Ripgrep(ripgrep::RipgrepInstaller),
    Bat(bat::BatInstaller),
    Fd(fd::FdInstaller),
    Eza(eza::EzaInstaller),
    Shellcheck(shellcheck::ShellcheckInstaller),
    NerdFont(nerd_font::NerdFontInstaller),
    JavaScript(javascript::JavaScriptInstaller),
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

/// All registered tools in installation order.
pub const ALL_TOOLS: &[Tool] = &[
    // Phase 0: base
    Tool::Brew(brew::BrewInstaller),
    Tool::Base(base::BaseInstaller),
    // Phase 1: parallel tools
    Tool::Go(go::GoInstaller),
    Tool::Rust(rust_lang::RustInstaller),
    Tool::Docker(docker::DockerInstaller),
    Tool::Azure(azure::AzureInstaller),
    Tool::Dotnet(dotnet::DotnetInstaller),
    Tool::Neovim(neovim::NeovimInstaller),
    Tool::Obsidian(obsidian::ObsidianInstaller),
    Tool::Java(java::JavaInstaller),
    Tool::Github(github::GithubCliInstaller),
    Tool::Terraform(terraform::TerraformInstaller),
    Tool::Postgres(postgres::PostgresInstaller),
    Tool::Kubectl(kubectl::KubectlInstaller),
    Tool::Ripgrep(ripgrep::RipgrepInstaller),
    Tool::Bat(bat::BatInstaller),
    Tool::Fd(fd::FdInstaller),
    Tool::Eza(eza::EzaInstaller),
    Tool::Shellcheck(shellcheck::ShellcheckInstaller),
    Tool::NerdFont(nerd_font::NerdFontInstaller),
    // Phase 2: JS sequential
    Tool::JavaScript(javascript::JavaScriptInstaller),
];

/// Return list of all available tool names.
pub fn available_tool_names() -> Vec<&'static str> {
    ALL_TOOLS.iter().map(|t| t.name()).collect()
}

/// Find a tool by name.
pub fn find_tool(name: &str) -> Option<Tool> {
    ALL_TOOLS.iter().copied().find(|t| t.name() == name)
}

/// Run a single installer by name.
pub fn run_by_name(name: &str, config: &InstallConfig) -> Result<()> {
    if name == "all" {
        return run_all(config);
    }

    let tool = match find_tool(name) {
        Some(t) => t,
        None => {
            println!("Unknown tool: {name}");
            println!("\nAvailable tools:");
            for tool_name in available_tool_names() {
                println!("  {tool_name}");
            }
            bail!("unknown tool: {name}");
        }
    };

    let outcome = run_one(&tool, config);
    print_single_outcome(tool.name(), &outcome);
    Ok(())
}

/// Run a single installer with pre-flight checks.
pub fn run_one(tool: &Tool, config: &InstallConfig) -> InstallOutcome {
    if tool.is_installed() {
        return InstallOutcome::Skipped("already installed".to_string());
    }

    if tool.needs_sudo(&config.platform) && !crate::common::command::is_root() {
        return InstallOutcome::Failed(format!(
            "requires sudo — re-run with: sudo bashc install {}",
            tool.name()
        ));
    }

    if config.dry_run {
        println!("Would install {}", tool.name());
        return InstallOutcome::Skipped("dry-run".to_string());
    }

    println!("\n--- Installing {} ---", tool.name());
    match tool.install(config) {
        Ok(()) => InstallOutcome::Installed,
        Err(e) => InstallOutcome::Failed(format!("{e:#}")),
    }
}

/// Run all installers with phased parallel execution.
pub fn run_all(config: &InstallConfig) -> Result<()> {
    // Pre-flight: check sudo requirements
    if !config.dry_run {
        let needs_sudo: Vec<&str> = ALL_TOOLS
            .iter()
            .filter(|t| t.needs_sudo(&config.platform) && !crate::common::command::is_root())
            .map(|t| t.name())
            .collect();

        if !needs_sudo.is_empty() {
            bail!(
                "The following tools require sudo on this platform: {}\nRe-run with: sudo bashc install all",
                needs_sudo.join(", ")
            );
        }
    }

    // Group by phase
    let phase0: Vec<Tool> = ALL_TOOLS.iter().copied().filter(|t| t.phase() == 0).collect();
    let phase1: Vec<Tool> = ALL_TOOLS.iter().copied().filter(|t| t.phase() == 1).collect();
    let phase2: Vec<Tool> = ALL_TOOLS.iter().copied().filter(|t| t.phase() == 2).collect();

    let mut results: Vec<(String, InstallOutcome)> = Vec::new();

    // Phase 0: base packages (sequential — brew first, then apt base)
    if !phase0.is_empty() {
        println!("=== Phase 0: Base packages ===");
        for tool in &phase0 {
            let outcome = run_one(tool, config);
            results.push((tool.name().to_string(), outcome));
        }
    }

    // Phase 1: parallel tool installation
    if !phase1.is_empty() {
        println!("\n=== Phase 1: Tools (parallel) ===");
        let phase1_results = run_phase_parallel(&phase1, config);
        results.extend(phase1_results);
    }

    // Phase 2: JS tools (sequential — nvm first, then rest)
    if !phase2.is_empty() {
        println!("\n=== Phase 2: JavaScript tools ===");
        for tool in &phase2 {
            let outcome = run_one(tool, config);
            results.push((tool.name().to_string(), outcome));
        }
    }

    print_summary(&results);
    Ok(())
}

/// Run a set of tools in parallel using tokio::task::spawn_blocking.
fn run_phase_parallel(
    tools: &[Tool],
    config: &InstallConfig,
) -> Vec<(String, InstallOutcome)> {
    // For dry-run, just run sequentially
    if config.dry_run {
        return tools
            .iter()
            .map(|t| {
                let outcome = run_one(t, config);
                (t.name().to_string(), outcome)
            })
            .collect();
    }

    let rt = tokio::runtime::Handle::current();
    let shared_config = Arc::new(ConfigSnapshot {
        platform: config.platform,
        dry_run: config.dry_run,
        verbose: config.verbose,
        interactive: config.interactive,
    });

    let mut handles = Vec::new();

    for &tool in tools {
        let name = tool.name().to_string();

        // Pre-flight checks before spawning
        if tool.is_installed() {
            handles.push((
                name,
                None,
                Some(InstallOutcome::Skipped("already installed".to_string())),
            ));
            continue;
        }

        if tool.needs_sudo(&shared_config.platform) && !crate::common::command::is_root() {
            handles.push((
                name.clone(),
                None,
                Some(InstallOutcome::Failed(format!(
                    "requires sudo — re-run with: sudo bashc install {name}"
                ))),
            ));
            continue;
        }

        let task_config = Arc::clone(&shared_config);
        // Tool is Copy — no Box needed, just move the value into the closure
        let handle = rt.spawn_blocking(move || {
            println!("\n--- Installing {} ---", tool.name());
            let install_config = InstallConfig {
                platform: task_config.platform,
                dry_run: task_config.dry_run,
                verbose: task_config.verbose,
                interactive: task_config.interactive,
            };
            match tool.install(&install_config) {
                Ok(()) => InstallOutcome::Installed,
                Err(e) => InstallOutcome::Failed(format!("{e:#}")),
            }
        });

        handles.push((name, Some(handle), None));
    }

    // Collect results
    let mut results = Vec::new();
    for (name, handle, immediate) in handles {
        if let Some(outcome) = immediate {
            results.push((name, outcome));
        } else if let Some(handle) = handle {
            let outcome = rt.block_on(handle).unwrap_or_else(|e| {
                InstallOutcome::Failed(format!("task panicked: {e}"))
            });
            results.push((name, outcome));
        }
    }

    results
}

/// Snapshot of InstallConfig that can be shared across threads.
struct ConfigSnapshot {
    platform: Platform,
    dry_run: bool,
    verbose: bool,
    interactive: bool,
}

/// Interactive mode: show multi-select menu.
pub fn run_interactive(config: &InstallConfig) -> Result<()> {
    let names: Vec<&str> = ALL_TOOLS.iter().map(|t| t.name()).collect();
    let selections = dialoguer::MultiSelect::new()
        .with_prompt("Select tools to install")
        .items(&names)
        .interact()?;

    if selections.is_empty() {
        println!("Nothing selected.");
        return Ok(());
    }

    let mut results: Vec<(String, InstallOutcome)> = Vec::new();
    for idx in selections {
        let tool = &ALL_TOOLS[idx];
        let outcome = run_one(tool, config);
        results.push((tool.name().to_string(), outcome));
    }

    print_summary(&results);
    Ok(())
}

fn print_single_outcome(name: &str, outcome: &InstallOutcome) {
    match outcome {
        InstallOutcome::Installed => println!("✓ {name} installed successfully"),
        InstallOutcome::Skipped(reason) => println!("- {name} skipped ({reason})"),
        InstallOutcome::Failed(reason) => println!("✗ {name} failed: {reason}"),
    }
}

fn print_summary(results: &[(String, InstallOutcome)]) {
    let installed: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::Installed))
        .collect();
    let skipped: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::Skipped(_)))
        .collect();
    let failed: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::Failed(_)))
        .collect();

    let total = results.len();
    let success = installed.len();

    println!("\n{}", "=".repeat(50));
    println!("Installed {success}/{total} tools successfully.\n");

    if !skipped.is_empty() {
        println!("Skipped:");
        for (name, outcome) in &skipped {
            if let InstallOutcome::Skipped(reason) = outcome {
                println!("  {name} — {reason}");
            }
        }
        println!();
    }

    if !failed.is_empty() {
        println!("Failed:");
        for (name, outcome) in &failed {
            if let InstallOutcome::Failed(reason) = outcome {
                println!("  {name} — {reason}");
            }
        }
        println!("\nFailed tools can be retried individually: bashc install <tool>");
    }
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
        // 20 tools + base = 21
        assert_eq!(ALL_TOOLS.len(), 21, "expected 21 tools (20 + base)");
    }

    #[test]
    fn tool_is_copy() {
        // Compile-time proof that Tool is Copy
        let t = ALL_TOOLS[0];
        let _t2 = t;
        let _t3 = t; // still valid — t was copied, not moved
    }
}
