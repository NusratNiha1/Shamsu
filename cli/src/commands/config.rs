use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use colored::Colorize;

use crate::storage;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// List all configuration keys and values
    List,
    /// Get a single config value
    Get {
        /// Config key name
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key name
        key: String,
        /// Config value
        value: String,
    },
    /// Delete a config key
    Unset {
        /// Config key name
        key: String,
    },
}

/// Known config keys and their descriptions
const KNOWN_KEYS: &[(&str, &str)] = &[
    ("llm_url", "llama.cpp server URL, e.g. http://127.0.0.1:8080"),
    ("temperature", "LLM sampling temperature (0.0–2.0), default 0.7"),
    ("max_tokens", "Max tokens per response, default 2048"),
    ("default_profile", "Default permission profile: safe | standard | full"),
    ("stream", "Stream responses: true | false"),
];

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.action {
        ConfigAction::List => {
            let entries = storage::list_config()?;
            println!("\n{}", "── Configuration ─────────────────────────────────".dimmed());
            if entries.is_empty() {
                println!("  {}", "No configuration set. Using defaults.".dimmed());
            }
            for (k, v) in &entries {
                let desc = KNOWN_KEYS
                    .iter()
                    .find(|(key, _)| key == k)
                    .map(|(_, d)| format!("  {}", d.dimmed()))
                    .unwrap_or_default();
                println!("  {} = {}{}", k.bold(), v.bright_cyan(), desc);
            }

            // Show unset keys with defaults
            println!("\n  {}", "Defaults (not yet set):".dimmed());
            let set_keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
            for (key, desc) in KNOWN_KEYS {
                if !set_keys.contains(key) {
                    println!("  {} — {}", key.dimmed(), desc.dimmed());
                }
            }
            println!("{}", "──────────────────────────────────────────────────".dimmed());
        }

        ConfigAction::Get { key } => {
            match storage::get_config(&key)? {
                Some(value) => println!("{} = {}", key.bold(), value.bright_cyan()),
                None => println!("  {} Key '{}' is not set.", "i".bright_blue(), key),
            }
        }

        ConfigAction::Set { key, value } => {
            // Basic validation
            match key.as_str() {
                "temperature" => {
                    let v: f32 = value.parse().map_err(|_| anyhow::anyhow!("temperature must be a float"))?;
                    if !(0.0..=2.0).contains(&v) {
                        bail!("temperature must be between 0.0 and 2.0");
                    }
                }
                "max_tokens" => {
                    value.parse::<i32>().map_err(|_| anyhow::anyhow!("max_tokens must be an integer"))?;
                }
                "default_profile" => {
                    if !["safe", "standard", "full"].contains(&value.as_str()) {
                        bail!("default_profile must be: safe | standard | full");
                    }
                }
                "stream" => {
                    if !["true", "false"].contains(&value.as_str()) {
                        bail!("stream must be: true | false");
                    }
                }
                _ => {}
            }
            storage::set_config(&key, &value)?;
            println!("  {} Set {} = {}", "✓".bright_green(), key.bold(), value.bright_cyan());
        }

        ConfigAction::Unset { key } => {
            let conn = storage::open()?;
            conn.execute("DELETE FROM config WHERE key = ?1", rusqlite::params![key])?;
            println!("  {} Removed key '{}'", "✓".bright_green(), key.bold());
        }
    }

    Ok(())
}
