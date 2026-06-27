/// MCP (Model Context Protocol) client — built-in tool implementations.
///
/// Built-in tools available to the assistant:
///   - read_file   — reads a file from disk
///   - write_file  — writes content to a file (permission-gated)
///   - list_dir    — lists the contents of a directory
///   - run_shell   — executes a shell command (permission-gated)
///   - search_files — grep-style search across workspace files

use anyhow::{anyhow, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use crate::permissions::Permissions;

// ─── Tool definitions ─────────────────────────────────────────────────────────

/// Describes a single callable tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Returns the list of built-in tools in JSON-schema format
pub fn builtin_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "read_file".to_string(),
            description: "Read the full contents of a file from disk.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or workspace-relative file path" }
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "write_file".to_string(),
            description: "Write content to a file. Creates directories if needed.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to write" },
                    "content": { "type": "string", "description": "Content to write" }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDef {
            name: "list_dir".to_string(),
            description: "List the files and folders in a directory.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to list" }
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "run_shell".to_string(),
            description: "Execute a shell command. Requires 'full' permission profile.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute" },
                    "cwd": { "type": "string", "description": "Working directory (optional)" }
                },
                "required": ["command"]
            }),
        },
        ToolDef {
            name: "search_files".to_string(),
            description: "Search for a pattern across files in a directory.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Text pattern to search" },
                    "directory": { "type": "string", "description": "Directory to search in" },
                    "file_pattern": { "type": "string", "description": "Glob-like file filter, e.g. *.rs" }
                },
                "required": ["pattern", "directory"]
            }),
        },
    ]
}

// ─── Tool execution ───────────────────────────────────────────────────────────

/// Result of a tool call
#[derive(Debug)]
pub struct ToolResult {
    pub tool_name: String,
    #[allow(dead_code)]
    pub success: bool,
    pub output: String,
}

/// Dispatch a tool call by name
pub async fn call_tool(
    name: &str,
    args: &Value,
    permissions: &Permissions,
) -> ToolResult {
    let result = match name {
        "read_file" => tool_read_file(args).await,
        "write_file" => tool_write_file(args, permissions).await,
        "list_dir" => tool_list_dir(args).await,
        "run_shell" => tool_run_shell(args, permissions).await,
        "search_files" => tool_search_files(args).await,
        _ => Err(anyhow!("Unknown tool: {}", name)),
    };

    match result {
        Ok(output) => {
            println!("  {} {} → OK", "Tool".bright_magenta().bold(), name.bold());
            ToolResult { tool_name: name.to_string(), success: true, output }
        }
        Err(e) => {
            println!(
                "  {} {} → {}",
                "Tool".bright_magenta().bold(),
                name.bold(),
                format!("Error: {}", e).red()
            );
            ToolResult {
                tool_name: name.to_string(),
                success: false,
                output: format!("Error: {}", e),
            }
        }
    }
}

async fn tool_read_file(args: &Value) -> Result<String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'path'"))?;
    let content = std::fs::read_to_string(Path::new(path))
        .map_err(|e| anyhow!("Cannot read '{}': {}", path, e))?;
    Ok(content)
}

async fn tool_write_file(args: &Value, permissions: &Permissions) -> Result<String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'path'"))?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'content'"))?;

    permissions.can_write(path)?;

    if permissions.dry_run {
        return Ok(format!(
            "[dry-run] Would write {} bytes to '{}'",
            content.len(),
            path
        ));
    }

    // Create parent dirs if needed
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(format!("Wrote {} bytes to '{}'", content.len(), path))
}

async fn tool_list_dir(args: &Value) -> Result<String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'path'"))?;

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(Path::new(path))
        .map_err(|e| anyhow!("Cannot list '{}': {}", path, e))?
        .flatten()
    {
        let meta = entry.metadata().ok();
        let is_dir = meta.map(|m| m.is_dir()).unwrap_or(false);
        let name = entry.file_name().to_string_lossy().to_string();
        if is_dir {
            entries.push(format!("{}/", name));
        } else {
            entries.push(name);
        }
    }
    entries.sort();
    Ok(entries.join("\n"))
}

async fn tool_run_shell(args: &Value, permissions: &Permissions) -> Result<String> {
    permissions.can_execute_shell()?;

    let command = args["command"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'command'"))?;
    let cwd = args["cwd"].as_str();

    if permissions.dry_run {
        return Ok(format!("[dry-run] Would run: {}", command));
    }

    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", command]);
        c
    };

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .map_err(|e| anyhow!("Failed to execute '{}': {}", command, e))?;

    let mut result = String::new();
    if !output.stdout.is_empty() {
        result.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        result.push_str("\nstderr:\n");
        result.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    if result.is_empty() {
        result.push_str("(no output)");
    }
    Ok(result)
}

async fn tool_search_files(args: &Value) -> Result<String> {
    let pattern = args["pattern"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'pattern'"))?;
    let directory = args["directory"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing 'directory'"))?;
    let file_pattern = args["file_pattern"].as_str();

    let mut matches: Vec<String> = Vec::new();
    search_recursive(
        Path::new(directory),
        pattern,
        file_pattern,
        &mut matches,
        0,
    )?;

    if matches.is_empty() {
        Ok(format!("No matches found for '{}'", pattern))
    } else {
        Ok(matches.join("\n"))
    }
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    file_pattern: Option<&str>,
    matches: &mut Vec<String>,
    depth: usize,
) -> Result<()> {
    if depth > 6 || matches.len() >= 100 {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs and common noise
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }

        if path.is_dir() {
            search_recursive(&path, pattern, file_pattern, matches, depth + 1)?;
        } else if path.is_file() {
            // File pattern filter
            if let Some(fp) = file_pattern {
                let ext = fp.trim_start_matches('*').trim_start_matches('.');
                if !name.ends_with(ext) {
                    continue;
                }
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&pattern.to_lowercase()) {
                        matches.push(format!(
                            "{}:{}: {}",
                            path.to_string_lossy(),
                            i + 1,
                            line.trim()
                        ));
                        if matches.len() >= 100 {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Format the tool definitions as a system prompt section so the LLM
/// knows what tools are available.
pub fn tools_system_prompt(tools: &[ToolDef]) -> String {
    let mut out = String::from(
        "\n\n## Available Tools\n\
         You can call tools by responding with a JSON block in this format:\n\
         ```tool_call\n\
         {\"tool\": \"<name>\", \"args\": {<arguments>}}\n\
         ```\n\
         Available tools:\n",
    );
    for tool in tools {
        out.push_str(&format!("- **{}**: {}\n", tool.name, tool.description));
    }
    out
}

/// Try to extract a tool call from an LLM response
pub fn extract_tool_call(response: &str) -> Option<(String, Value)> {
    // Look for ```tool_call ... ``` blocks
    if let Some(start) = response.find("```tool_call") {
        let rest = &response[start + 12..];
        if let Some(end) = rest.find("```") {
            let json_str = rest[..end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                if let (Some(tool), Some(args)) = (v["tool"].as_str(), v.get("args")) {
                    return Some((tool.to_string(), args.clone()));
                }
            }
        }
    }
    // Also check for plain JSON with "tool" key on a single line
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                if let (Some(tool), Some(args)) = (v["tool"].as_str(), v.get("args")) {
                    return Some((tool.to_string(), args.clone()));
                }
            }
        }
    }
    None
}
