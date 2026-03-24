mod common;
mod configs;
mod install;

use clap::{Parser, Subcommand};

use configs::Strategy;

#[derive(Parser)]
#[command(name = "bashc", version, about = "Unified CLI for shell customization")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install development tools and languages
    Install {
        /// Tool to install (or "all" for everything)
        tool: Option<String>,

        /// Show interactive multi-select menu
        #[arg(long)]
        interactive: bool,

        /// Show what would be done without executing
        #[arg(long)]
        dry_run: bool,

        /// Show full subprocess output
        #[arg(long)]
        verbose: bool,
    },

    /// Manage symlinked config files (claude, zellij, ghostty, etc.)
    Configs {
        #[command(subcommand)]
        action: ConfigsAction,
    },
}

#[derive(Subcommand)]
enum ConfigsAction {
    /// Create symlinks from repo configs to system locations
    Link {
        /// Config group name (e.g. "claude", "zellij"). Links all if omitted.
        name: Option<String>,

        /// Force a specific conflict resolution strategy (replace, discard, keep)
        #[arg(long, value_enum)]
        force: Option<Strategy>,
    },

    /// Remove symlinks and optionally restore backups
    Unlink {
        /// Config group name. Unlinks all if omitted.
        name: Option<String>,

        /// Skip confirmation prompts (answer yes to all)
        #[arg(long)]
        yes: bool,
    },

    /// Show current state of all managed configs
    Status {
        /// Config group name. Shows all if omitted.
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            tool,
            interactive,
            dry_run,
            verbose,
        } => {
            let platform = common::platform::Platform::detect()?;
            println!("Detected platform: {}", platform);

            let config = install::InstallConfig {
                platform,
                dry_run,
                verbose,
                interactive,
            };

            if interactive {
                install::run_interactive(&config)?;
            } else if let Some(tool_name) = tool {
                install::run_by_name(&tool_name, &config)?;
            } else {
                println!("Usage: bashc install <tool> or bashc install --interactive");
                println!("\nAvailable tools:");
                for name in install::available_tool_names() {
                    println!("  {name}");
                }
            }
        }
        Commands::Configs { action } => {
            let platform = common::platform::Platform::detect()?;
            let project_root = common::project_root::project_root()?;

            match action {
                ConfigsAction::Link { name, force } => {
                    configs::link::run_link(
                        &project_root,
                        &platform,
                        name.as_deref(),
                        force,
                    )?;
                }
                ConfigsAction::Unlink { name, yes } => {
                    configs::unlink::run_unlink(
                        &project_root,
                        &platform,
                        name.as_deref(),
                        yes,
                    )?;
                }
                ConfigsAction::Status { name } => {
                    configs::status::run_status(
                        &project_root,
                        &platform,
                        name.as_deref(),
                    )?;
                }
            }
        }
    }

    Ok(())
}
