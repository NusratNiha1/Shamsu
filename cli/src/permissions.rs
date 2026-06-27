use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

/// Permission profiles — how much the assistant is allowed to do
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionProfile {
    /// Read-only: no file writes, no shell
    Safe,
    /// File read/write in workspace; no shell
    Standard,
    /// File read/write anywhere + shell execution
    Full,
}

impl PermissionProfile {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "safe" => PermissionProfile::Safe,
            "full" => PermissionProfile::Full,
            _ => PermissionProfile::Standard,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionProfile::Safe => "safe",
            PermissionProfile::Standard => "standard",
            PermissionProfile::Full => "full",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PermissionProfile::Safe => "Read-only. No file writes, no shell.",
            PermissionProfile::Standard => "Read + write files in workspace. No shell.",
            PermissionProfile::Full => "Read + write files anywhere. Shell execution enabled.",
        }
    }
}

/// Runtime permission gate
pub struct Permissions {
    pub profile: PermissionProfile,
    pub workspace_root: String,
    pub dry_run: bool,
}

impl Permissions {
    pub fn new(profile_str: &str, workspace_root: &str, dry_run: bool) -> Self {
        Permissions {
            profile: PermissionProfile::from_str(profile_str),
            workspace_root: workspace_root.to_string(),
            dry_run,
        }
    }

    /// Check whether reading a file is allowed (always yes)
    #[allow(dead_code)]
    pub fn can_read(&self, _path: &str) -> bool {
        true
    }

    /// Check whether writing a file is allowed
    pub fn can_write(&self, path: &str) -> Result<()> {
        if self.profile == PermissionProfile::Safe {
            bail!(
                "Write permission denied (profile: safe). \
                 Switch to 'standard' or 'full' with `shamsu session set-profile <name> standard`."
            );
        }
        if self.profile == PermissionProfile::Standard {
            // Must be inside workspace
            let abs = std::fs::canonicalize(Path::new(path))
                .unwrap_or_else(|_| Path::new(path).to_path_buf());
            let root = std::fs::canonicalize(Path::new(&self.workspace_root))
                .unwrap_or_else(|_| Path::new(&self.workspace_root).to_path_buf());
            if !abs.starts_with(&root) {
                bail!(
                    "Write permission denied: `{}` is outside the workspace. \
                     Use 'full' profile to write outside.",
                    path
                );
            }
        }
        Ok(())
    }

    /// Check whether shell execution is allowed
    pub fn can_execute_shell(&self) -> Result<()> {
        if self.profile != PermissionProfile::Full {
            bail!(
                "Shell execution denied (profile: {}). \
                 Switch to 'full' with `shamsu session set-profile <name> full`.",
                self.profile.as_str()
            );
        }
        Ok(())
    }

    /// Print permission profile info
    pub fn print_summary(&self) {
        let color = match self.profile {
            PermissionProfile::Safe => "safe".green(),
            PermissionProfile::Standard => "standard".yellow(),
            PermissionProfile::Full => "full".red(),
        };
        println!(
            "  {} {}  {}",
            "Permissions:".bold(),
            color,
            format!("({})", self.profile.description()).dimmed()
        );
        if self.dry_run {
            println!("  {} {}", "Mode:".bold(), "dry-run (no changes written)".bright_yellow());
        }
    }
}

/// Prompt the user to pick a permission profile interactively
#[allow(dead_code)]
pub fn prompt_profile() -> PermissionProfile {
    println!("\n{}", "Select a permission profile:".bold());
    println!("  [1] {} — {}", "safe".green(), "Read-only. No file writes.");
    println!("  [2] {} — {}", "standard".yellow(), "Read + write files in workspace. (default)");
    println!("  [3] {} — {}", "full".red(), "Read + write + shell execution.");
    print!("  Choice [2]: ");

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    match input.trim() {
        "1" => PermissionProfile::Safe,
        "3" => PermissionProfile::Full,
        _ => PermissionProfile::Standard,
    }
}
