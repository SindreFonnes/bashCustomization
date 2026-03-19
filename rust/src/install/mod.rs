mod azure;
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
    all_installers().iter().map(|i| i.name().to_string()).collect()
}

/// Return all registered installers.
pub fn all_installers() -> Vec<Box<dyn Installer>> {
    vec![
        // Phase 0: base
        Box::new(brew::BrewInstaller),
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

    match installer.install(config) {
        Ok(()) => InstallOutcome::Installed,
        Err(e) => InstallOutcome::Failed(format!("{e:#}")),
    }
}

/// Run all installers sequentially with summary.
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

    let mut results: Vec<(&str, InstallOutcome)> = Vec::new();

    for installer in &installers {
        let outcome = run_one(installer.as_ref(), config);
        results.push((installer.name(), outcome));
    }

    print_summary(&results);
    Ok(())
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

    let mut results: Vec<(&str, InstallOutcome)> = Vec::new();
    for idx in selections {
        let installer = &installers[idx];
        let outcome = run_one(installer.as_ref(), config);
        results.push((installer.name(), outcome));
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

fn print_summary(results: &[(&str, InstallOutcome)]) {
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
    println!(
        "Installed {success}/{total} tools successfully.\n"
    );

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
