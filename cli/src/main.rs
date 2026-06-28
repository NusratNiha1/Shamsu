mod commands;
mod context;
mod extractor;
mod llm;
mod mcp;
mod permissions;
mod skills;
mod storage;
mod ui;
mod workspace;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "shamsu",
    about = "Shamsu — Offline-first AI developer assistant",
    version = "0.1.0",
    long_about = "An offline-first, privacy-preserving AI coding assistant powered by local LLMs."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Auto-approve all confirmations (no prompts)
    #[arg(long, global = true)]
    yes: bool,

    /// Enable verbose/debug output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to workspace directory (defaults to current dir)
    #[arg(short, long, global = true)]
    workspace: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session
    Chat(commands::chat::ChatArgs),

    /// Manage sessions (list, new, switch, delete, export)
    Session(commands::session::SessionArgs),

    /// Manage skills
    Skills(commands::skills::SkillsArgs),

    /// Show and edit Shamsu configuration
    Config(commands::config::ConfigArgs),

    /// Detect and display workspace information
    Inspect,

    /// Check llama.cpp server connectivity
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Print banner on direct invocation
    if cli.command.is_none() {
        ui::print_banner();
        println!("Run {} for usage.", "shamsu --help".cyan());
        return Ok(());
    }

    // Initialize storage (creates DB + dirs if needed)
    storage::init().await?;

    let workspace_path = cli
        .workspace
        .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());

    match cli.command.unwrap() {
        Commands::Chat(args) => {
            commands::chat::run(args, &workspace_path, cli.yes, cli.verbose).await?
        }
        Commands::Session(args) => commands::session::run(args).await?,
        Commands::Skills(args) => commands::skills::run(args).await?,
        Commands::Config(args) => commands::config::run(args).await?,
        Commands::Inspect => {
            let info = workspace::detect(&workspace_path).await?;
            workspace::print_info(&info);
        }
        Commands::Status => {
            commands::status::check().await?;
        }
    }

    Ok(())
}

