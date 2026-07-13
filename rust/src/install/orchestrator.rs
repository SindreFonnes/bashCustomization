use anyhow::{Result, bail};

use super::{
    ALL_TOOLS, InstallConfig, InstallOutcome, Installer, Tool, available_tool_names, find_tool,
};

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
    match outcome {
        InstallOutcome::Failed(reason) => {
            bail!("installation of {} failed: {}", tool.name(), reason);
        }
        _ => Ok(()),
    }
}

/// Run a single installer with pre-flight checks.
fn run_one(tool: &Tool, config: &InstallConfig) -> InstallOutcome {
    if !tool.is_applicable(&config.platform) {
        return InstallOutcome::NotApplicable(format!("not applicable on {}", config.platform));
    }

    if config.platform.is_nixos() {
        if config.dry_run {
            println!(
                "  Would provide NixOS declarative guidance for {}",
                tool.name()
            );
            return InstallOutcome::Planned;
        }

        return match crate::common::package_manager::nix_guidance(tool.name()) {
            Ok(()) => InstallOutcome::Guidance("declarative NixOS configuration".to_string()),
            Err(e) => InstallOutcome::Failed(format!("{e:#}")),
        };
    }

    if config.dry_run {
        return match tool.install(config) {
            Ok(()) => InstallOutcome::Planned,
            Err(e) => InstallOutcome::Failed(format!("{e:#}")),
        };
    }

    if tool.is_installed() {
        return InstallOutcome::Skipped("already installed".to_string());
    }

    if tool.needs_sudo(&config.platform)
        && !crate::common::command::is_root()
        && !crate::common::privilege::has_path_escalator()
    {
        return InstallOutcome::Failed(format!(
            "requires root privileges — no sudo/doas/su found to install {}",
            tool.name()
        ));
    }

    println!("\n--- Installing {} ---", tool.name());
    match tool.install(config) {
        Ok(()) => InstallOutcome::Installed,
        Err(e) => InstallOutcome::Failed(format!("{e:#}")),
    }
}

/// Run all installers in dependency phases.
pub fn run_all(config: &InstallConfig) -> Result<()> {
    // Pre-flight: check sudo requirements
    if !config.dry_run {
        let needs_sudo: Vec<&str> = ALL_TOOLS
            .iter()
            .filter(|t| {
                t.include_in_all(&config.platform)
                    && t.needs_sudo(&config.platform)
                    && !crate::common::command::is_root()
                    && !crate::common::privilege::has_path_escalator()
            })
            .map(|t| t.name())
            .collect();

        if !needs_sudo.is_empty() {
            bail!(
                "The following tools require root privileges and no sudo/doas/su was found: {}\nInstall sudo/doas or re-run as root",
                needs_sudo.join(", ")
            );
        }
    }

    // Pre-flight: bootstrap doas on Alpine when running as root with no escalator
    //
    // On a fresh Alpine install there is no sudo/doas/su, so tools that rely on
    // privilege escalation would fail later. If we are already root we can
    // install doas right now (best-effort — if it fails we warn and continue;
    // tools that need escalation will produce their own clear error messages).
    if !config.dry_run
        && crate::common::command::is_root()
        && !crate::common::privilege::has_path_escalator()
        && config.platform.is_alpine()
    {
        println!("=== Pre-flight: bootstrapping doas on Alpine ===");
        if let Some(doas_tool) = find_tool("doas") {
            match doas_tool.install(config) {
                Ok(()) => println!("doas bootstrapped successfully."),
                Err(e) => println!(
                    "Warning: doas bootstrap failed ({e:#}). \
                    Tools requiring privilege escalation may fail."
                ),
            }
        }
    }

    // Group by phase
    let phase0: Vec<Tool> = ALL_TOOLS
        .iter()
        .copied()
        .filter(|t| t.phase() == 0)
        .collect();
    let phase1: Vec<Tool> = ALL_TOOLS
        .iter()
        .copied()
        .filter(|t| t.phase() == 1)
        .collect();
    let phase2: Vec<Tool> = ALL_TOOLS
        .iter()
        .copied()
        .filter(|t| t.phase() == 2)
        .collect();

    let mut results: Vec<(String, InstallOutcome)> = Vec::new();

    // Phase 0: base packages (sequential — brew first, then apt base)
    if !phase0.is_empty() {
        println!("=== Phase 0: Base packages ===");
        for tool in &phase0 {
            let outcome = if tool.include_in_all(&config.platform) {
                run_one(tool, config)
            } else {
                InstallOutcome::NotApplicable(
                    "not required by install all on this platform".to_string(),
                )
            };
            results.push((tool.name().to_string(), outcome));
        }
    }

    // Phase 1: tools. Keep this sequential: many installers mutate global
    // package-manager state (apt/brew locks, repo files, /usr/local), so
    // concurrent installs are not reliable across platforms.
    if !phase1.is_empty() {
        println!("\n=== Phase 1: Tools ===");
        for tool in &phase1 {
            let outcome = if tool.include_in_all(&config.platform) {
                run_one(tool, config)
            } else {
                InstallOutcome::NotApplicable(
                    "not required by install all on this platform".to_string(),
                )
            };
            results.push((tool.name().to_string(), outcome));
        }
    }

    // Phase 2: JS tools (sequential — nvm first, then rest)
    if !phase2.is_empty() {
        println!("\n=== Phase 2: JavaScript tools ===");
        for tool in &phase2 {
            let outcome = if tool.include_in_all(&config.platform) {
                run_one(tool, config)
            } else {
                InstallOutcome::NotApplicable(
                    "not required by install all on this platform".to_string(),
                )
            };
            results.push((tool.name().to_string(), outcome));
        }
    }

    print_summary(&results);
    bail_if_failed(&results)
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
    bail_if_failed(&results)
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn print_single_outcome(name: &str, outcome: &InstallOutcome) {
    match outcome {
        InstallOutcome::Installed => println!("✓ {name} installed successfully"),
        InstallOutcome::Skipped(reason) => println!("- {name} skipped ({reason})"),
        InstallOutcome::NotApplicable(reason) => {
            println!("- {name} not applicable ({reason})")
        }
        InstallOutcome::Guidance(reason) => println!("- {name} guidance provided ({reason})"),
        InstallOutcome::Planned => println!("- {name} planned (dry-run)"),
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
    let not_applicable: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::NotApplicable(_)))
        .collect();
    let planned: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::Planned))
        .collect();
    let guidance: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::Guidance(_)))
        .collect();
    let failed: Vec<_> = results
        .iter()
        .filter(|(_, o)| matches!(o, InstallOutcome::Failed(_)))
        .collect();

    let applicable =
        installed.len() + skipped.len() + guidance.len() + planned.len() + failed.len();
    let completed = installed.len() + skipped.len() + guidance.len();

    println!("\n{}", "=".repeat(50));
    if !planned.is_empty() {
        println!(
            "Planned {}/{} applicable tools.\n",
            planned.len(),
            applicable
        );
    } else {
        println!("Completed {completed}/{applicable} applicable tools successfully.\n");
    }

    if !installed.is_empty() {
        println!("Installed:");
        for (name, _) in &installed {
            println!("  {name}");
        }
        println!();
    }

    if !skipped.is_empty() {
        println!("Skipped:");
        for (name, outcome) in &skipped {
            if let InstallOutcome::Skipped(reason) = outcome {
                println!("  {name} — {reason}");
            }
        }
        println!();
    }

    if !not_applicable.is_empty() {
        println!("Not applicable:");
        for (name, outcome) in &not_applicable {
            if let InstallOutcome::NotApplicable(reason) = outcome {
                println!("  {name} — {reason}");
            }
        }
        println!();
    }

    if !guidance.is_empty() {
        println!("Guidance provided:");
        for (name, outcome) in &guidance {
            if let InstallOutcome::Guidance(reason) = outcome {
                println!("  {name} — {reason}");
            }
        }
        println!();
    }

    if !planned.is_empty() {
        println!("Planned:");
        for (name, _) in &planned {
            println!("  {name}");
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

fn bail_if_failed(results: &[(String, InstallOutcome)]) -> Result<()> {
    let failed: Vec<&str> = results
        .iter()
        .filter_map(|(name, outcome)| {
            matches!(outcome, InstallOutcome::Failed(_)).then_some(name.as_str())
        })
        .collect();

    if failed.is_empty() {
        Ok(())
    } else {
        bail!("{} tool(s) failed: {}", failed.len(), failed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os, Platform};

    fn test_config(dry_run: bool) -> InstallConfig {
        InstallConfig {
            platform: Platform {
                os: Os::Linux(Distro::Debian),
                arch: Arch::X86_64,
            },
            dry_run,
        }
    }

    #[test]
    fn run_by_name_unknown_tool_returns_error() {
        let config = test_config(false);
        let result = run_by_name("nonexistent_tool_xyz", &config);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("unknown tool"),
            "expected 'unknown tool' in error: {msg}"
        );
    }

    #[test]
    fn run_by_name_dry_run_succeeds() {
        let config = test_config(true);
        // dry-run should always succeed (tools are skipped, not executed)
        let result = run_by_name("ripgrep", &config);
        assert!(result.is_ok(), "dry-run should not fail: {result:?}");
    }

    #[test]
    fn run_one_skips_installed_tool() {
        // Go is likely not installed in the test env, but if it is, it should skip
        let config = test_config(false);
        for &tool in ALL_TOOLS {
            if tool.is_installed() {
                let outcome = run_one(&tool, &config);
                assert!(
                    matches!(outcome, InstallOutcome::Skipped(_)),
                    "installed tool {} should be skipped",
                    tool.name()
                );
                break;
            }
        }
    }

    #[test]
    fn run_one_dry_run_plans() {
        let config = test_config(true);
        let tool = find_tool("ripgrep").unwrap();
        let outcome = run_one(&tool, &config);
        assert!(matches!(outcome, InstallOutcome::Planned));
    }

    #[test]
    fn doas_is_not_applicable_on_macos() {
        let config = InstallConfig {
            platform: Platform {
                os: Os::MacOs,
                arch: Arch::Aarch64,
            },
            dry_run: true,
        };
        let tool = find_tool("doas").unwrap();

        assert!(matches!(
            run_one(&tool, &config),
            InstallOutcome::NotApplicable(_)
        ));
    }

    #[test]
    fn brew_is_not_applicable_on_alpine() {
        let config = InstallConfig {
            platform: Platform {
                os: Os::Linux(Distro::Alpine),
                arch: Arch::X86_64,
            },
            dry_run: true,
        };
        let tool = find_tool("brew").unwrap();

        assert!(matches!(
            run_one(&tool, &config),
            InstallOutcome::NotApplicable(_)
        ));
    }

    #[test]
    fn nixos_install_returns_guidance_outcome() {
        let config = InstallConfig {
            platform: Platform {
                os: Os::Linux(Distro::NixOs),
                arch: Arch::X86_64,
            },
            dry_run: false,
        };
        let tool = find_tool("docker").unwrap();

        assert!(matches!(
            run_one(&tool, &config),
            InstallOutcome::Guidance(_)
        ));
    }

    #[test]
    fn bail_if_failed_returns_error_for_failed_outcomes() {
        let results = vec![
            ("ripgrep".to_string(), InstallOutcome::Installed),
            (
                "docker".to_string(),
                InstallOutcome::Failed("apt lock failed".to_string()),
            ),
        ];

        let err = bail_if_failed(&results).expect_err("failed outcomes should error");
        let msg = err.to_string();
        assert!(
            msg.contains("docker"),
            "error should name failed tool: {msg}"
        );
    }
}
