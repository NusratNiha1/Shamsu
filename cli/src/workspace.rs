use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::fmt;

/// Describes what kind of workspace was detected
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceKind {
    /// An existing codebase (has source files and/or git history)
    ExistingProject,
    /// A requirements/spec document was found — build from it
    RequirementsDoc { doc_path: String },
    /// Blank workspace — start from a text prompt
    Blank,
}

impl fmt::Display for WorkspaceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceKind::ExistingProject => write!(f, "Existing Project"),
            WorkspaceKind::RequirementsDoc { doc_path } => {
                write!(f, "Requirements Document ({})", doc_path)
            }
            WorkspaceKind::Blank => write!(f, "Blank Workspace"),
        }
    }
}

/// Information about the current workspace
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub path: String,
    pub kind: WorkspaceKind,
    pub detected_stack: Vec<String>,
    pub has_git: bool,
    pub file_count: usize,
    pub key_files: Vec<String>,
}

/// Detect the workspace state in the given directory
pub async fn detect(workspace_path: &str) -> Result<WorkspaceInfo> {
    let path = Path::new(workspace_path);

    let has_git = path.join(".git").exists();
    let mut detected_stack: Vec<String> = Vec::new();
    let mut key_files: Vec<String> = Vec::new();
    let mut file_count = 0;
    let mut req_doc: Option<String> = None;

    // Requirements/spec document names to look for
    let req_doc_names = [
        "requirements.md",
        "Requirements.md",
        "REQUIREMENTS.md",
        "prd.md",
        "PRD.md",
        "spec.md",
        "SPEC.md",
        "spec.txt",
        "requirements.txt",
        "project-description.md",
        "Project description.md",
        "DESIGN.md",
    ];

    // Stack indicator files
    let stack_indicators: &[(&str, &str)] = &[
        ("package.json", "Node.js/JavaScript"),
        ("Cargo.toml", "Rust"),
        ("pyproject.toml", "Python"),
        ("requirements.txt", "Python"),
        ("go.mod", "Go"),
        ("pom.xml", "Java/Maven"),
        ("build.gradle", "Java/Gradle"),
        ("CMakeLists.txt", "C/C++"),
        ("Makefile", "C/C++/Make"),
        ("composer.json", "PHP"),
        ("Gemfile", "Ruby"),
        ("mix.exs", "Elixir"),
        ("pubspec.yaml", "Dart/Flutter"),
        ("tsconfig.json", "TypeScript"),
    ];

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();

            if entry.path().is_file() {
                file_count += 1;

                // Check for requirements docs
                if req_doc.is_none() {
                    for req_name in &req_doc_names {
                        if name_str.eq_ignore_ascii_case(req_name) {
                            req_doc = Some(entry.path().to_string_lossy().to_string());
                            break;
                        }
                    }
                }

                // Check for stack indicators
                for (indicator, stack) in stack_indicators {
                    if name_str == *indicator {
                        let stack_str = stack.to_string();
                        if !detected_stack.contains(&stack_str) {
                            detected_stack.push(stack_str);
                        }
                        key_files.push(name_str.clone());
                    }
                }
            } else if entry.path().is_dir() {
                // Count source dirs as indicator of existing project
                let src_dirs = ["src", "lib", "app", "pkg", "cmd", "internal"];
                if src_dirs.contains(&name_str.as_str()) {
                    file_count += 10; // weight source dirs
                    key_files.push(format!("{}/", name_str));
                }
            }
        }
    }

    // Determine workspace kind
    let kind = if let Some(doc_path) = req_doc {
        WorkspaceKind::RequirementsDoc { doc_path }
    } else if has_git || file_count > 3 || !detected_stack.is_empty() {
        WorkspaceKind::ExistingProject
    } else {
        WorkspaceKind::Blank
    };

    Ok(WorkspaceInfo {
        path: workspace_path.to_string(),
        kind,
        detected_stack,
        has_git,
        file_count,
        key_files,
    })
}

/// Print workspace info to the terminal
pub fn print_info(info: &WorkspaceInfo) {
    println!("\n{}", "── Workspace Info ──────────────────────────────".dimmed());
    println!("  {} {}", "Path:".bold(), info.path);
    println!("  {} {}", "Kind:".bold(), info.kind.to_string().bright_cyan());
    println!("  {} {}", "Git:".bold(), if info.has_git { "yes".green() } else { "no".dimmed() });
    println!("  {} {}", "Files:".bold(), info.file_count);

    if !info.detected_stack.is_empty() {
        println!("  {} {}", "Stack:".bold(), info.detected_stack.join(", ").bright_yellow());
    }

    if !info.key_files.is_empty() {
        println!("  {} {}", "Key files:".bold(), info.key_files.join(", ").dimmed());
    }
    println!("{}", "────────────────────────────────────────────────".dimmed());
}

/// Get a short context description to inject into the system prompt
pub fn context_description(info: &WorkspaceInfo) -> String {
    match &info.kind {
        WorkspaceKind::ExistingProject => {
            let mut desc = format!(
                "You are working inside an existing project at `{}`.",
                info.path
            );
            if !info.detected_stack.is_empty() {
                desc.push_str(&format!(" Detected stack: {}.", info.detected_stack.join(", ")));
            }
            if info.has_git {
                desc.push_str(" The project has git history.");
            }
            desc
        }
        WorkspaceKind::RequirementsDoc { doc_path } => {
            format!(
                "A requirements document has been found at `{}`. \
                 Read it and use it to guide your work.",
                doc_path
            )
        }
        WorkspaceKind::Blank => {
            format!(
                "You are in a blank workspace at `{}`. \
                 When given a prompt, first generate architecture documentation, \
                 then implement step by step.",
                info.path
            )
        }
    }
}
