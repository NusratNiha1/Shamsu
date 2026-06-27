/// mcp.rs — Built-in agentic tool implementations.
///
/// Tools:
///   read_file    — read a file
///   write_file   — write/create a file (permission-gated, prompts user)
///   patch_file   — replace a specific string inside a file (targeted edit)
///   delete_file  — delete a file (permission-gated, prompts user)
///   create_dir   — create a directory tree
///   list_dir     — list directory contents
///   run_shell    — execute shell command (full profile only, prompts user)
///   search_files — grep-style recursive search

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use crate::permissions::{PermissionProfile, Permissions};
use crate::ui::{self, ToolStatus};

// ─── Tool definitions ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub fn builtin_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "read_file".into(),
            description: "Read the full contents of a file from disk.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type":"string","description":"Absolute or workspace-relative file path"}
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "write_file".into(),
            description: "Write content to a file, creating it (and any parent directories) if needed. \
                          For targeted edits to existing files prefer patch_file.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path":    {"type":"string","description":"File path to write"},
                    "content": {"type":"string","description":"Full file content to write"}
                },
                "required": ["path","content"]
            }),
        },
        ToolDef {
            name: "patch_file".into(),
            description: "Replace an exact string inside an existing file. \
                          Use this for targeted edits instead of rewriting the whole file.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path":       {"type":"string","description":"File to patch"},
                    "old_str":    {"type":"string","description":"Exact text to find and replace"},
                    "new_str":    {"type":"string","description":"Replacement text"}
                },
                "required": ["path","old_str","new_str"]
            }),
        },
        ToolDef {
            name: "delete_file".into(),
            description: "Delete a file from disk. Requires user confirmation.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type":"string","description":"File path to delete"}
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "create_dir".into(),
            description: "Create a directory (and any missing parents).".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type":"string","description":"Directory path to create"}
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "list_dir".into(),
            description: "List files and folders in a directory.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type":"string","description":"Directory path to list"}
                },
                "required": ["path"]
            }),
        },
        ToolDef {
            name: "run_shell".into(),
            description: "Execute a shell command and return its output. \
                          Requires 'full' permission profile and user approval.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type":"string","description":"Shell command to execute"},
                    "cwd":     {"type":"string","description":"Working directory (optional)"}
                },
                "required": ["command"]
            }),
        },
        ToolDef {
            name: "search_files".into(),
            description: "Search for a text pattern across files in a directory.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern":      {"type":"string","description":"Text pattern to search for"},
                    "directory":    {"type":"string","description":"Directory to search in"},
                    "file_pattern": {"type":"string","description":"Optional file filter e.g. *.rs"}
                },
                "required": ["pattern","directory"]
            }),
        },
    ]
}

// ─── Tool result ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ToolResult {
    pub tool_name: String,
    #[allow(dead_code)]
    pub success: bool,
    pub output: String,
}

// ─── Dispatcher ───────────────────────────────────────────────────────────────

pub async fn call_tool(
    name: &str,
    args: &Value,
    permissions: &Permissions,
    auto_yes: bool,
) -> ToolResult {
    ui::print_tool_card(name, arg_summary(name, args).as_str(), ToolStatus::Running);

    let result = match name {
        "read_file"    => tool_read_file(args).await,
        "write_file"   => tool_write_file(args, permissions, auto_yes).await,
        "patch_file"   => tool_patch_file(args, permissions, auto_yes).await,
        "delete_file"  => tool_delete_file(args, permissions, auto_yes).await,
        "create_dir"   => tool_create_dir(args, permissions, auto_yes).await,
        "list_dir"     => tool_list_dir(args).await,
        "run_shell"    => tool_run_shell(args, permissions, auto_yes).await,
        "search_files" => tool_search_files(args).await,
        _ => Err(anyhow!("Unknown tool: {}", name)),
    };

    // Reprint card with final status (overwrite last line via carriage return isn't reliable
    // in all terminals, so we print a fresh status line)
    match &result {
        Ok(out) => {
            ui::print_tool_card(name, &first_line(out, 50), ToolStatus::Ok);
            ToolResult { tool_name: name.into(), success: true, output: out.clone() }
        }
        Err(e) => {
            ui::print_tool_card(name, &e.to_string(), ToolStatus::Err);
            ToolResult { tool_name: name.into(), success: false, output: format!("Error: {e}") }
        }
    }
}

// ─── Individual tools ─────────────────────────────────────────────────────────

async fn tool_read_file(args: &Value) -> Result<String> {
    let path = str_arg(args, "path")?;
    let content = std::fs::read_to_string(Path::new(path))
        .map_err(|e| anyhow!("Cannot read '{}': {}", path, e))?;
    Ok(content)
}

async fn tool_write_file(args: &Value, perms: &Permissions, auto_yes: bool) -> Result<String> {
    let path    = str_arg(args, "path")?;
    let content = str_arg(args, "content")?;

    // Permission gate
    perms.can_write(path)?;

    // Prompt unless auto_yes
    if !auto_yes && perms.profile != PermissionProfile::Full {
        let preview = content.lines().take(5).collect::<Vec<_>>().join("\n");
        if !ui::prompt_permission("write_file", path, Some(&preview)) {
            return Ok("[Skipped by user]".into());
        }
    } else if !auto_yes {
        let preview = content.lines().take(5).collect::<Vec<_>>().join("\n");
        if !ui::prompt_permission("write_file", path, Some(&preview)) {
            return Ok("[Skipped by user]".into());
        }
    }

    if perms.dry_run {
        return Ok(format!("[dry-run] Would write {} bytes to '{}'", content.len(), path));
    }

    let is_new = !Path::new(path).exists();
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    let lines = content.lines().count();
    ui::print_file_written(path, content.len(), lines, is_new);
    Ok(format!("Wrote {} bytes ({} lines) to '{}'", content.len(), lines, path))
}

async fn tool_patch_file(args: &Value, perms: &Permissions, auto_yes: bool) -> Result<String> {
    let path    = str_arg(args, "path")?;
    let old_str = str_arg(args, "old_str")?;
    let new_str = str_arg(args, "new_str")?;

    perms.can_write(path)?;

    let original = std::fs::read_to_string(Path::new(path))
        .map_err(|e| anyhow!("Cannot read '{}': {}", path, e))?;

    if !original.contains(old_str) {
        return Err(anyhow!("patch_file: '{}' — old_str not found in file", path));
    }

    // Show a mini diff
    let detail = format!("- {}\n+ {}", old_str.lines().next().unwrap_or(""), new_str.lines().next().unwrap_or(""));
    if !auto_yes && !ui::prompt_permission("patch_file", path, Some(&detail)) {
        return Ok("[Skipped by user]".into());
    }

    if perms.dry_run {
        return Ok(format!("[dry-run] Would patch '{}'", path));
    }

    let patched = original.replacen(old_str, new_str, 1);
    std::fs::write(path, &patched)?;
    let lines = patched.lines().count();
    ui::print_file_written(path, patched.len(), lines, false);
    Ok(format!("Patched '{}' ({} lines)", path, lines))
}

async fn tool_delete_file(args: &Value, perms: &Permissions, auto_yes: bool) -> Result<String> {
    let path = str_arg(args, "path")?;

    perms.can_write(path)?;

    if !auto_yes && !ui::prompt_permission("delete_file", path, None) {
        return Ok("[Skipped by user]".into());
    }

    if perms.dry_run {
        return Ok(format!("[dry-run] Would delete '{}'", path));
    }

    std::fs::remove_file(Path::new(path))
        .map_err(|e| anyhow!("Cannot delete '{}': {}", path, e))?;
    ui::print_file_deleted(path);
    Ok(format!("Deleted '{}'", path))
}

async fn tool_create_dir(args: &Value, perms: &Permissions, auto_yes: bool) -> Result<String> {
    let path = str_arg(args, "path")?;

    // treat like write for permission purposes
    if perms.profile == PermissionProfile::Safe {
        return Err(anyhow!("create_dir denied (safe profile)"));
    }

    if !auto_yes && !ui::prompt_permission("create_dir", path, None) {
        return Ok("[Skipped by user]".into());
    }

    if perms.dry_run {
        return Ok(format!("[dry-run] Would create directory '{}'", path));
    }

    std::fs::create_dir_all(Path::new(path))
        .map_err(|e| anyhow!("Cannot create dir '{}': {}", path, e))?;
    ui::print_success(&format!("Created directory '{}'", path));
    Ok(format!("Created directory '{}'", path))
}

async fn tool_list_dir(args: &Value) -> Result<String> {
    let path = str_arg(args, "path")?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(Path::new(path))
        .map_err(|e| anyhow!("Cannot list '{}': {}", path, e))?
        .flatten()
    {
        let meta  = entry.metadata().ok();
        let is_dir = meta.map(|m| m.is_dir()).unwrap_or(false);
        let name  = entry.file_name().to_string_lossy().to_string();
        entries.push(if is_dir { format!("{}/", name) } else { name });
    }
    entries.sort();
    Ok(entries.join("\n"))
}

async fn tool_run_shell(args: &Value, perms: &Permissions, auto_yes: bool) -> Result<String> {
    perms.can_execute_shell()?;

    let command = str_arg(args, "command")?;
    let cwd     = args["cwd"].as_str();

    ui::print_shell_block(command, cwd);

    if !auto_yes && !ui::prompt_permission("run_shell", command, cwd.map(|c| format!("cwd: {}", c)).as_deref()) {
        return Ok("[Skipped by user]".into());
    }

    if perms.dry_run {
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

    let output = cmd.output()
        .map_err(|e| anyhow!("Failed to execute '{}': {}", command, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    let combined = if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{}\nstderr:\n{}", stdout, stderr)
    };

    ui::print_shell_output(&combined, success);

    if combined.is_empty() {
        Ok(format!("(exit {})", output.status.code().unwrap_or(0)))
    } else {
        Ok(combined)
    }
}

async fn tool_search_files(args: &Value) -> Result<String> {
    let pattern   = str_arg(args, "pattern")?;
    let directory = str_arg(args, "directory")?;
    let file_pat  = args["file_pattern"].as_str();

    let mut matches = Vec::new();
    search_recursive(Path::new(directory), pattern, file_pat, &mut matches, 0)?;

    if matches.is_empty() {
        Ok(format!("No matches for '{}'", pattern))
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
    if depth > 8 || matches.len() >= 200 { return Ok(()); }
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name == "target" { continue; }
        if path.is_dir() {
            search_recursive(&path, pattern, file_pattern, matches, depth + 1)?;
        } else if path.is_file() {
            if let Some(fp) = file_pattern {
                let ext = fp.trim_start_matches('*').trim_start_matches('.');
                if !name.ends_with(ext) { continue; }
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&pattern.to_lowercase()) {
                        matches.push(format!("{}:{}: {}", path.to_string_lossy(), i + 1, line.trim()));
                        if matches.len() >= 200 { return Ok(()); }
                    }
                }
            }
        }
    }
    Ok(())
}

// ─── System prompt for tools ──────────────────────────────────────────────────

pub fn tools_system_prompt(tools: &[ToolDef]) -> String {
    let mut out = String::from(
        "\n\n## Agentic Tool Use\n\
You are an agentic coding assistant. When the user asks you to create, edit, \
delete, or run things, USE THE TOOLS — do not just show code in markdown. \
Always use tools to make actual changes.\n\
\n\
Emit tool calls using this exact JSON format inside a fenced block:\n\
```tool_call\n\
{\"tool\": \"<name>\", \"args\": {<arguments>}}\n\
```\n\
\n\
You may emit multiple tool calls in sequence. Always read a file before editing it \
unless you are creating it fresh. Use patch_file for small edits, write_file for \
new files or full rewrites.\n\
\n\
After all tool calls are done, summarise what you did in plain text.\n\
\n\
Available tools:\n",
    );
    for t in tools {
        out.push_str(&format!("- **{}**: {}\n", t.name, t.description));
    }
    out
}

// ─── Tool call extraction ─────────────────────────────────────────────────────

/// Returns ALL tool calls found in a response (not just the first)
pub fn extract_all_tool_calls(response: &str) -> Vec<(String, Value)> {
    let mut calls = Vec::new();
    let mut search = response;

    while let Some(start) = search.find("```tool_call") {
        let rest = &search[start + 12..];
        if let Some(end) = rest.find("```") {
            let json_str = rest[..end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                if let (Some(tool), Some(args)) = (v["tool"].as_str(), v.get("args")) {
                    calls.push((tool.to_string(), args.clone()));
                }
            }
            search = &rest[end + 3..];
        } else {
            break;
        }
    }

    // Also scan for bare JSON lines with "tool" key (fallback)
    if calls.is_empty() {
        for line in response.lines() {
            let t = line.trim();
            if t.starts_with('{') && t.contains("\"tool\"") {
                if let Ok(v) = serde_json::from_str::<Value>(t) {
                    if let (Some(tool), Some(args)) = (v["tool"].as_str(), v.get("args")) {
                        calls.push((tool.to_string(), args.clone()));
                    }
                }
            }
        }
    }

    calls
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().ok_or_else(|| anyhow!("Missing required argument '{}'", key))
}

fn first_line(s: &str, max: usize) -> String {
    let line = s.lines().next().unwrap_or(s);
    if line.len() > max { format!("{}…", &line[..max]) } else { line.to_string() }
}

fn arg_summary(tool: &str, args: &Value) -> String {
    match tool {
        "read_file" | "write_file" | "patch_file" | "delete_file" | "create_dir" | "list_dir" =>
            args["path"].as_str().unwrap_or("").to_string(),
        "run_shell"    => args["command"].as_str().unwrap_or("").to_string(),
        "search_files" => format!("{} in {}", args["pattern"].as_str().unwrap_or(""), args["directory"].as_str().unwrap_or("")),
        _ => String::new(),
    }
}
