use std::sync::Arc;

use anyhow::{Result, bail};

use crate::common::platform::Platform;
use super::{InstallConfig, InstallOutcome, Installer, Tool, ALL_TOOLS, available_tool_names, find_tool};

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
fn run_one(tool: &Tool, config: &InstallConfig) -> InstallOutcome {
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

// ---------------------------------------------------------------------------
// Parallel execution
// ---------------------------------------------------------------------------

/// Snapshot of InstallConfig that can be shared across threads.
struct ConfigSnapshot {
    platform: Platform,
    dry_run: bool,
    verbose: bool,
    interactive: bool,
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
        platform: config.platform.clone(),
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
        // Tool is Copy — just move the value into the closure
        let handle = rt.spawn_blocking(move || {
            println!("\n--- Installing {} ---", tool.name());
            let install_config = InstallConfig {
                platform: task_config.platform.clone(),
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

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

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
