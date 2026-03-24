mod common;
mod configs;
mod install;

use clap::{Parser, Subcommand};

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
    }

    Ok(())
}
