use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A skill is a named system-prompt module stored as a YAML or Markdown file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub tags: Vec<String>,
}

impl Skill {
    fn skills_dir() -> PathBuf {
        crate::storage::shamsu_dir().join("skills")
    }

    /// Load all available skills from ~/.shamsu/skills/
    pub fn list_all() -> Result<Vec<Skill>> {
        let dir = Self::skills_dir();
        let mut skills = Vec::new();

        if !dir.exists() {
            return Ok(skills);
        }

        for entry in std::fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy();
                if ext == "yaml" || ext == "yml" || ext == "md" {
                    if let Ok(skill) = Self::load_from_path(&path) {
                        skills.push(skill);
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Load a skill by name
    pub fn load(name: &str) -> Result<Option<Skill>> {
        let dir = Self::skills_dir();
        let candidates = [
            dir.join(format!("{}.yaml", name)),
            dir.join(format!("{}.yml", name)),
            dir.join(format!("{}.md", name)),
        ];
        for path in &candidates {
            if path.exists() {
                return Ok(Some(Self::load_from_path(path)?));
            }
        }
        Ok(None)
    }

    fn load_from_path(path: &std::path::Path) -> Result<Skill> {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        if ext == "md" {
            // Markdown: first H1 = name, first paragraph = description, rest = prompt
            let content = std::fs::read_to_string(path)?;
            let mut lines = content.lines();
            let name = lines
                .next()
                .unwrap_or("")
                .trim_start_matches('#')
                .trim()
                .to_string();
            let description = lines.next().unwrap_or("").trim().to_string();
            let prompt = lines.collect::<Vec<_>>().join("\n").trim().to_string();
            Ok(Skill {
                name,
                description,
                prompt,
                tags: vec![],
            })
        } else {
            // YAML
            let content = std::fs::read_to_string(path)?;
            Ok(serde_yaml::from_str(&content)?)
        }
    }

    /// Save a skill to disk
    pub fn save(&self) -> Result<()> {
        let dir = Self::skills_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.yaml", self.name));
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }
}

/// Load the combined system-prompt text for a list of active skill names
pub fn build_skills_prompt(skill_names: &[String]) -> String {
    if skill_names.is_empty() {
        return String::new();
    }
    let mut combined = String::from("\n\n--- Active Skills ---\n");
    for name in skill_names {
        match Skill::load(name) {
            Ok(Some(skill)) => {
                combined.push_str(&format!("\n[Skill: {}]\n{}\n", skill.name, skill.prompt));
            }
            Ok(None) => {
                eprintln!(
                    "{}",
                    format!("Warning: skill '{}' not found, skipping.", name).yellow()
                );
            }
            Err(e) => {
                eprintln!("{}", format!("Warning: failed to load skill '{}': {}", name, e).yellow());
            }
        }
    }
    combined
}

/// Seed built-in skills on first run
pub fn seed_builtin_skills() -> Result<()> {
    let builtins: &[(&str, &str, &str)] = &[
        (
            "coding",
            "General coding assistant",
            "You are an expert software engineer. Write clean, well-commented, idiomatic code. \
             Prefer simple solutions over complex ones. Always explain your changes.",
        ),
        (
            "write-tests",
            "Test generation specialist",
            "Focus on writing thorough unit and integration tests. Cover edge cases, error paths, \
             and happy paths. Use the project's existing test framework.",
        ),
        (
            "refactor",
            "Code refactoring expert",
            "Identify and improve code quality: reduce duplication, improve naming, simplify logic, \
             improve error handling. Explain each change and why it is better.",
        ),
        (
            "explain",
            "Code explainer for learning",
            "Explain code in simple, clear language. Use analogies where helpful. \
             Break down complex concepts step by step.",
        ),
        (
            "docs",
            "Documentation writer",
            "Write clear, accurate documentation. Include usage examples, parameter descriptions, \
             and return value explanations. Follow the project's existing doc style.",
        ),
    ];

    for (name, description, prompt) in builtins {
        let skill = Skill {
            name: name.to_string(),
            description: description.to_string(),
            prompt: prompt.to_string(),
            tags: vec![],
        };
        // Only write if not already present
        let dir = Skill::skills_dir();
        let path = dir.join(format!("{}.yaml", name));
        if !path.exists() {
            skill.save()?;
        }
    }

    Ok(())
}

/// Print a skill list to stdout
pub fn print_skill_list(skills: &[Skill]) {
    if skills.is_empty() {
        println!("  {}", "No skills found. Add .yaml or .md files to ~/.shamsu/skills/".dimmed());
        return;
    }
    println!("\n{}", "── Available Skills ─────────────────────────────".dimmed());
    for skill in skills {
        let tags = if skill.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", skill.tags.join(", ")).dimmed().to_string()
        };
        println!(
            "  {} {} {}{}",
            "•".bright_cyan(),
            skill.name.bold(),
            skill.description.dimmed(),
            tags
        );
    }
    println!("{}", "─────────────────────────────────────────────────".dimmed());
}
