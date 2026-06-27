use anyhow::{anyhow, bail, Result};
use clap::{Args, Subcommand};
use colored::Colorize;

use crate::skills::{self, Skill};
use crate::storage;

#[derive(Args)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub action: SkillsAction,
}

#[derive(Subcommand)]
pub enum SkillsAction {
    /// List all available skills
    List,
    /// Show the prompt content of a skill
    Show {
        /// Skill name
        name: String,
    },
    /// Activate a skill for the current session
    Activate {
        /// Skill name
        name: String,
    },
    /// Deactivate a skill from the current session
    Deactivate {
        /// Skill name
        name: String,
    },
    /// Create a new skill interactively
    New {
        /// Skill name (slug, no spaces)
        name: String,
        /// One-line description
        #[arg(short, long)]
        description: Option<String>,
    },
}

pub async fn run(args: SkillsArgs) -> Result<()> {
    match args.action {
        SkillsAction::List => {
            let all = Skill::list_all()?;
            skills::print_skill_list(&all);

            // Show which are active in current session
            if let Ok(Some(session)) = storage::get_active_session() {
                if !session.active_skills.is_empty() {
                    println!(
                        "  {} {}",
                        "Active in current session:".bold(),
                        session.active_skills.join(", ").bright_magenta()
                    );
                }
            }
        }

        SkillsAction::Show { name } => {
            let skill = Skill::load(&name)?
                .ok_or_else(|| anyhow!("Skill '{}' not found", name))?;
            println!("\n{}", format!("── Skill: {} ──", skill.name).bright_cyan());
            println!("{}", skill.description.dimmed());
            println!("{}", "─────────────────────────────────────────────────".dimmed());
            println!("{}", skill.prompt);
        }

        SkillsAction::Activate { name } => {
            // Verify skill exists
            Skill::load(&name)?
                .ok_or_else(|| anyhow!("Skill '{}' not found. Use `shamsu skills list` to see available skills.", name))?;

            let session = storage::get_active_session()?
                .ok_or_else(|| anyhow!("No active session. Create one with `shamsu session new <name>`"))?;

            let mut active = session.active_skills.clone();
            if active.contains(&name) {
                println!("  {} Skill '{}' is already active.", "i".bright_blue(), name);
                return Ok(());
            }
            active.push(name.clone());
            let skills_json = serde_json::to_string(&active)?;

            let conn = storage::open()?;
            conn.execute(
                "UPDATE sessions SET active_skills = ?1 WHERE id = ?2",
                rusqlite::params![skills_json, session.id],
            )?;
            println!("  {} Activated skill '{}' for session '{}'", "✓".bright_green(), name.bold(), session.name);
        }

        SkillsAction::Deactivate { name } => {
            let session = storage::get_active_session()?
                .ok_or_else(|| anyhow!("No active session."))?;

            let active: Vec<String> = session
                .active_skills
                .into_iter()
                .filter(|s| s != &name)
                .collect();
            let skills_json = serde_json::to_string(&active)?;

            let conn = storage::open()?;
            conn.execute(
                "UPDATE sessions SET active_skills = ?1 WHERE id = ?2",
                rusqlite::params![skills_json, session.id],
            )?;
            println!("  {} Deactivated skill '{}' for session '{}'", "✓".bright_green(), name, session.name);
        }

        SkillsAction::New { name, description } => {
            // Validate name
            if name.contains(' ') {
                bail!("Skill name cannot contain spaces. Use a slug like 'my-skill'.");
            }
            if Skill::load(&name)?.is_some() {
                bail!("A skill named '{}' already exists.", name);
            }

            let desc = match description {
                Some(d) => d,
                None => {
                    print!("  Description: ");
                    let mut d = String::new();
                    std::io::stdin().read_line(&mut d)?;
                    d.trim().to_string()
                }
            };

            println!("  Enter the skill prompt (type END on a new line when done):");
            let mut prompt_lines: Vec<String> = Vec::new();
            loop {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
                if trimmed == "END" {
                    break;
                }
                prompt_lines.push(trimmed);
            }

            let skill = Skill {
                name: name.clone(),
                description: desc,
                prompt: prompt_lines.join("\n"),
                tags: vec![],
            };
            skill.save()?;
            println!("  {} Skill '{}' saved to ~/.shamsu/skills/{}.yaml", "✓".bright_green(), name.bold(), name);
        }
    }
    Ok(())
}
