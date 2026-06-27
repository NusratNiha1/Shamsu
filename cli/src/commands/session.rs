use anyhow::{anyhow, bail, Result};
use clap::{Args, Subcommand};
use colored::Colorize;

use crate::storage::{self, Session};

#[derive(Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub action: SessionAction,
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// List all sessions
    List,
    /// Create a new session
    New {
        /// Session name
        name: String,
        /// Workspace path (defaults to current directory)
        #[arg(short, long)]
        workspace: Option<String>,
        /// Permission profile: safe | standard | full
        #[arg(short, long, default_value = "standard")]
        profile: String,
    },
    /// Switch to a session by name
    Switch {
        /// Session name to activate
        name: String,
    },
    /// Delete a session by name
    Delete {
        /// Session name to delete
        name: String,
    },
    /// Rename a session
    Rename {
        /// Current session name
        name: String,
        /// New name
        new_name: String,
    },
    /// Set the permission profile for a session
    SetProfile {
        /// Session name
        name: String,
        /// Profile: safe | standard | full
        profile: String,
    },
    /// Show details about the active session
    Current,
    /// Export session messages to a text file
    Export {
        /// Session name
        name: String,
        /// Output file path (defaults to <name>.txt)
        #[arg(short, long)]
        output: Option<String>,
    },
}

pub async fn run(args: SessionArgs) -> Result<()> {
    match args.action {
        SessionAction::List => {
            let sessions = storage::list_sessions()?;
            if sessions.is_empty() {
                println!("  {}", "No sessions yet. Create one with `shamsu session new <name>`".dimmed());
                return Ok(());
            }
            println!("\n{}", "── Sessions ──────────────────────────────────────".dimmed());
            for s in &sessions {
                let active_marker = if s.is_active {
                    " ◀ active".bright_green().to_string()
                } else {
                    String::new()
                };
                println!(
                    "  {} {}  [{}]  {}{}",
                    "•".bright_cyan(),
                    s.name.bold(),
                    s.permission_profile.dimmed(),
                    s.workspace.dimmed(),
                    active_marker
                );
            }
            println!("{}", "──────────────────────────────────────────────────".dimmed());
        }

        SessionAction::New { name, workspace, profile } => {
            // Check for duplicate name
            if storage::get_session_by_name(&name)?.is_some() {
                bail!("A session named '{}' already exists.", name);
            }
            let ws = workspace.unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            });
            let mut session = Session::new(&name, &ws);
            session.permission_profile = profile.clone();
            storage::create_session(&session)?;
            storage::set_active_session(&session.id)?;
            println!(
                "  {} Created and activated session {} (profile: {})",
                "✓".bright_green(),
                name.bold(),
                profile.bright_yellow()
            );
        }

        SessionAction::Switch { name } => {
            let session = storage::get_session_by_name(&name)?
                .ok_or_else(|| anyhow!("No session named '{}'", name))?;
            storage::set_active_session(&session.id)?;
            println!(
                "  {} Switched to session {}",
                "✓".bright_green(),
                name.bold()
            );
        }

        SessionAction::Delete { name } => {
            let session = storage::get_session_by_name(&name)?
                .ok_or_else(|| anyhow!("No session named '{}'", name))?;
            storage::delete_session(&session.id)?;
            println!(
                "  {} Deleted session {}",
                "✓".bright_green(),
                name.bold()
            );
        }

        SessionAction::Rename { name, new_name } => {
            let session = storage::get_session_by_name(&name)?
                .ok_or_else(|| anyhow!("No session named '{}'", name))?;
            storage::rename_session(&session.id, &new_name)?;
            println!(
                "  {} Renamed '{}' → '{}'",
                "✓".bright_green(),
                name,
                new_name.bold()
            );
        }

        SessionAction::SetProfile { name, profile } => {
            let valid = ["safe", "standard", "full"];
            if !valid.contains(&profile.as_str()) {
                bail!("Invalid profile '{}'. Choose: safe | standard | full", profile);
            }
            let session = storage::get_session_by_name(&name)?
                .ok_or_else(|| anyhow!("No session named '{}'", name))?;

            // We update via raw SQL since we don't have a dedicated fn
            let conn = storage::open()?;
            conn.execute(
                "UPDATE sessions SET permission_profile = ?1 WHERE id = ?2",
                rusqlite::params![profile, session.id],
            )?;
            println!(
                "  {} Set profile for '{}' to {}",
                "✓".bright_green(),
                name.bold(),
                profile.bright_yellow()
            );
        }

        SessionAction::Current => {
            match storage::get_active_session()? {
                Some(s) => {
                    println!("\n{}", "── Active Session ────────────────────────────────".dimmed());
                    println!("  {} {}", "Name:".bold(), s.name.bright_cyan());
                    println!("  {} {}", "ID:".bold(), s.id.dimmed());
                    println!("  {} {}", "Workspace:".bold(), s.workspace);
                    println!("  {} {}", "Profile:".bold(), s.permission_profile.bright_yellow());
                    println!("  {} {}", "Created:".bold(), s.created_at.format("%Y-%m-%d %H:%M").to_string().dimmed());
                    println!("  {} {}", "Updated:".bold(), s.updated_at.format("%Y-%m-%d %H:%M").to_string().dimmed());
                    if !s.active_skills.is_empty() {
                        println!("  {} {}", "Skills:".bold(), s.active_skills.join(", ").bright_magenta());
                    }
                    println!("{}", "──────────────────────────────────────────────────".dimmed());
                }
                None => {
                    println!("  {}", "No active session. Create one: `shamsu session new <name>`".dimmed());
                }
            }
        }

        SessionAction::Export { name, output } => {
            let session = storage::get_session_by_name(&name)?
                .ok_or_else(|| anyhow!("No session named '{}'", name))?;
            let messages = storage::get_messages(&session.id, None)?;
            let out_path = output.unwrap_or_else(|| format!("{}.txt", name));

            let mut content = format!(
                "Shamsu Session Export: {}\nWorkspace: {}\nDate: {}\n\n",
                session.name,
                session.workspace,
                chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
            );
            for msg in &messages {
                content.push_str(&format!(
                    "─── {} ───\n{}\n\n",
                    msg.role.to_uppercase(),
                    msg.content
                ));
            }

            std::fs::write(&out_path, &content)?;
            println!(
                "  {} Exported {} messages to '{}'",
                "✓".bright_green(),
                messages.len(),
                out_path.bold()
            );
        }
    }

    Ok(())
}
