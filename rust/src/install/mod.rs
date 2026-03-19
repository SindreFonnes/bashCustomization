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
    /// Tool name used as the CLI argument.
    fn name(&self) -> &str;

    /// Whether this tool needs root on the given platform.
    fn needs_sudo(&self, platform: &Platform) -> bool;

    /// Check if already installed.
    fn is_installed(&self) -> bool;

    /// Perform the installation.
    fn install(&self, config: &InstallConfig) -> Result<()>;

    /// Installation phase: 0 = base, 1 = parallel tools, 2 = JS sequential.
    fn phase(&self) -> u8 {
        1
    }
}

/// Return list of all available tool names.
pub fn available_tool_names() -> Vec<String> {
    all_installers()
        .iter()
        .map(|i| i.name().to_string())
        .collect()
}

/// Return all registered installers.
pub fn all_installers() -> Vec<Box<dyn Installer>> {
    vec![
        // Phase 0: base
        Box::new(brew::BrewInstaller),
        Box::new(base::BaseInstaller),
        // Phase 1: parallel tools
        Box::new(go::GoInstaller),
        Box::new(rust_lang::RustInstaller),
        Box::new(docker::DockerInstaller),
        Box::new(azure::AzureInstaller),
        Box::new(dotnet::DotnetInstaller),
        Box::new(neovim::NeovimInstaller),
        Box::new(obsidian::ObsidianInstaller),
        Box::new(java::JavaInstaller),
        Box::new(github::GithubCliInstaller),
        Box::new(terraform::TerraformInstaller),
        Box::new(postgres::PostgresInstaller),
        Box::new(kubectl::KubectlInstaller),
        Box::new(ripgrep::RipgrepInstaller),
        Box::new(bat::BatInstaller),
        Box::new(fd::FdInstaller),
        Box::new(eza::EzaInstaller),
        Box::new(shellcheck::ShellcheckInstaller),
        Box::new(nerd_font::NerdFontInstaller),
        // Phase 2: JS sequential
        Box::new(javascript::JavaScriptInstaller),
    ]
}

/// Find an installer by name.
pub fn find_installer(name: &str) -> Option<Box<dyn Installer>> {
    all_installers().into_iter().find(|i| i.name() == name)
}

/// Run a single installer by name.
pub fn run_by_name(name: &str, config: &InstallConfig) -> Result<()> {
    if name == "all" {
        return run_all(config);
    }

    if name == "base" {
        let installer = base::BaseInstaller;
        let outcome = run_one(&installer, config);
        print_single_outcome(installer.name(), &outcome);
        return Ok(());
    }

    let installer = match find_installer(name) {
        Some(i) => i,
        None => {
            println!("Unknown tool: {name}");
            println!("\nAvailable tools:");
            for tool_name in available_tool_names() {
                println!("  {tool_name}");
            }
            bail!("unknown tool: {name}");
        }
    };

    let outcome = run_one(installer.as_ref(), config);
    print_single_outcome(installer.name(), &outcome);
    Ok(())
}

/// Run a single installer with pre-flight checks.
pub fn run_one(installer: &dyn Installer, config: &InstallConfig) -> InstallOutcome {
    if installer.is_installed() {
        return InstallOutcome::Skipped("already installed".to_string());
    }

    if installer.needs_sudo(&config.platform) && !crate::common::command::is_root() {
        return InstallOutcome::Failed(format!(
            "requires sudo — re-run with: sudo bashc install {}",
            installer.name()
        ));
    }

    if config.dry_run {
        println!("Would install {}", installer.name());
        return InstallOutcome::Skipped("dry-run".to_string());
    }

    println!("\n--- Installing {} ---", installer.name());
    match installer.install(config) {
        Ok(()) => InstallOutcome::Installed,
        Err(e) => InstallOutcome::Failed(format!("{e:#}")),
    }
}

/// Run all installers with phased parallel execution.
pub fn run_all(config: &InstallConfig) -> Result<()> {
    let installers = all_installers();

    if installers.is_empty() {
        println!("No installers registered yet.");
        return Ok(());
    }

    // Pre-flight: check sudo requirements
    if !config.dry_run {
        let needs_sudo: Vec<&str> = installers
            .iter()
            .filter(|i| i.needs_sudo(&config.platform) && !crate::common::command::is_root())
            .map(|i| i.name())
            .collect();

        if !needs_sudo.is_empty() {
            bail!(
                "The following tools require sudo on this platform: {}\nRe-run with: sudo bashc install all",
                needs_sudo.join(", ")
            );
        }
    }

    // Group installers by phase
    let phase0: Vec<_> = installers.iter().filter(|i| i.phase() == 0).collect();
    let phase1: Vec<_> = installers.iter().filter(|i| i.phase() == 1).collect();
    let phase2: Vec<_> = installers.iter().filter(|i| i.phase() == 2).collect();

    let mut results: Vec<(String, InstallOutcome)> = Vec::new();

    // Phase 0: base packages (sequential — brew first, then apt base)
    if !phase0.is_empty() {
        println!("=== Phase 0: Base packages ===");
        for installer in &phase0 {
            let outcome = run_one(installer.as_ref(), config);
            results.push((installer.name().to_string(), outcome));
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
        for installer in &phase2 {
            let outcome = run_one(installer.as_ref(), config);
            results.push((installer.name().to_string(), outcome));
        }
    }

    print_summary(&results);
    Ok(())
}

/// Run a set of installers in parallel using tokio::task::spawn_blocking.
fn run_phase_parallel(
    installers: &[&Box<dyn Installer>],
    config: &InstallConfig,
) -> Vec<(String, InstallOutcome)> {
    // For dry-run, just run sequentially (no real work to parallelize)
    if config.dry_run {
        return installers
            .iter()
            .map(|i| {
                let outcome = run_one(i.as_ref(), config);
                (i.name().to_string(), outcome)
            })
            .collect();
    }

    // Use tokio runtime for parallel execution
    let rt = tokio::runtime::Handle::current();
    let config = Arc::new(InstallConfigSnapshot {
        platform: config.platform,
        dry_run: config.dry_run,
        verbose: config.verbose,
        interactive: config.interactive,
    });

    let mut handles = Vec::new();

    for installer in installers {
        let name = installer.name().to_string();

        // Pre-flight checks before spawning
        if installer.is_installed() {
            handles.push((
                name,
                None,
                Some(InstallOutcome::Skipped("already installed".to_string())),
            ));
            continue;
        }

        if installer.needs_sudo(&config.platform) && !crate::common::command::is_root() {
            handles.push((
                name.clone(),
                None,
                Some(InstallOutcome::Failed(format!(
                    "requires sudo — re-run with: sudo bashc install {name}"
                ))),
            ));
            continue;
        }

        // Create a new config for the spawned task
        let task_config = Arc::clone(&config);
        let installer_ref = find_installer(&name).unwrap();

        let handle = rt.spawn_blocking(move || {
            println!("\n--- Installing {} ---", installer_ref.name());
            let install_config = InstallConfig {
                platform: task_config.platform,
                dry_run: task_config.dry_run,
                verbose: task_config.verbose,
                interactive: task_config.interactive,
            };
            match installer_ref.install(&install_config) {
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
struct InstallConfigSnapshot {
    platform: Platform,
    dry_run: bool,
    verbose: bool,
    interactive: bool,
}

/// Interactive mode: show multi-select menu.
pub fn run_interactive(config: &InstallConfig) -> Result<()> {
    let installers = all_installers();
    if installers.is_empty() {
        println!("No installers registered yet.");
        return Ok(());
    }

    let names: Vec<&str> = installers.iter().map(|i| i.name()).collect();
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
        let installer = &installers[idx];
        let outcome = run_one(installer.as_ref(), config);
        results.push((installer.name().to_string(), outcome));
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
    fn find_installer_known() {
        assert!(find_installer("go").is_some());
        assert!(find_installer("rust").is_some());
        assert!(find_installer("kubectl").is_some());
    }

    #[test]
    fn find_installer_unknown() {
        assert!(find_installer("nonexistent").is_none());
    }

    #[test]
    fn all_installers_has_20_plus_base() {
        let installers = all_installers();
        // 20 tools + base = 21
        assert_eq!(installers.len(), 21, "expected 21 installers (20 tools + base)");
    }
}
