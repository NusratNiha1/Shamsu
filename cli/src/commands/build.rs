/// commands/build.rs — Lovable/Bolt-style autonomous web app builder.
///
/// `shamsu build "a todo app with dark mode"`
/// `shamsu build --from requirements.md`
/// `shamsu build --from prd.md --workspace ./my-app`
///
/// How it works:
///   1. Reads the prompt (or a requirements doc)
///   2. Sends to the LLM with a powerful web-builder system prompt
///   3. Streams tokens live to screen
///   4. As tokens arrive, tool_call blocks are parsed on-the-fly
///   5. Tools are executed immediately (write_file, run_shell, etc.)
///   6. Tool results are fed back to the LLM — the loop continues
///   7. Up to MAX_AGENT_ITERATIONS iterations until the LLM stops calling tools
///   8. Auto-starts a dev server if one was set up

use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;

use crate::context;
use crate::llm::{LlmClient, StreamedToolCall};
use crate::mcp;
use crate::permissions::Permissions;
use crate::storage::{self, Message};
use crate::ui;
use crate::workspace;

/// Maximum agentic loop iterations (each iteration = one LLM call + tool execution)
const MAX_AGENT_ITERATIONS: usize = 20;

#[derive(Args)]
pub struct BuildArgs {
    /// What to build — a natural language description
    #[arg(index = 1)]
    pub prompt: Option<String>,

    /// Path to a requirements / PRD markdown file
    #[arg(short, long)]
    pub from: Option<String>,

    /// Session name to resume or create
    #[arg(short, long)]
    pub session: Option<String>,

    /// Skip the dev server auto-start at the end
    #[arg(long)]
    pub no_serve: bool,

    /// Skip the LLM server check at startup
    #[arg(long)]
    pub no_check: bool,
}

pub async fn run(args: BuildArgs, workspace_path: &str) -> Result<()> {
    // ── Validate we have something to build ───────────────────────────────
    if args.prompt.is_none() && args.from.is_none() {
        bail!(
            "Specify what to build:\n  shamsu build \"a todo app with dark mode\"\n  shamsu build --from requirements.md"
        );
    }

    // ── LLM check ─────────────────────────────────────────────────────────
    let llm = LlmClient::new();
    if !args.no_check && !llm.is_alive().await {
        ui::print_error(&format!(
            "Cannot reach llama.cpp server at {}\nRun `shamsu status` for help.",
            crate::llm::llm_url()
        ));
        return Ok(());
    }

    // ── Seed skills ───────────────────────────────────────────────────────
    let _ = crate::skills::seed_builtin_skills();

    // ── Session (always full profile for build) ───────────────────────────
    storage::init().await?;
    let session_name = args.session.clone().unwrap_or_else(|| {
        // derive a name from the prompt
        args.prompt.as_deref()
            .unwrap_or("build")
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join("-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
    });

    let session = {
        if let Some(s) = storage::get_session_by_name(&session_name)? {
            storage::set_active_session(&s.id)?;
            s
        } else {
            let mut s = crate::storage::Session::new(&session_name, workspace_path);
            s.permission_profile = "full".into();
            storage::create_session(&s)?;
            storage::set_active_session(&s.id)?;
            s
        }
    };
    let session_id = session.id.clone();

    // Build always uses full profile + auto_yes (no permission prompts)
    let permissions = Permissions::new("full", workspace_path, false);

    // ── Banner ────────────────────────────────────────────────────────────
    ui::print_banner();
    let model = llm.model_name().await.unwrap_or_else(|_| "offline".into());
    println!(
        "  {} {}  {}  {}",
        "⚡ BUILD MODE".bright_yellow().bold(),
        format!("session:{}", session_name).truecolor(180, 180, 200),
        format!("workspace:{}", workspace_path).truecolor(160, 160, 180),
        format!("model:{}", model).truecolor(140, 140, 180),
    );
    println!("  {}", "─".repeat(70).truecolor(50, 50, 65));
    println!();

    // ── Read the build request ────────────────────────────────────────────
    let user_request = build_user_request(&args, workspace_path)?;

    // ── System prompt ─────────────────────────────────────────────────────
    let ws_info = workspace::detect(workspace_path).await?;
    let system = build_system_prompt(workspace_path, &ws_info);

    // ── Print what we're building ─────────────────────────────────────────
    println!("  {} {}", "Building:".bright_cyan().bold(), user_request.lines().next().unwrap_or(""));
    println!();

    // ── Store the initial user message ────────────────────────────────────
    storage::append_message(&Message::new(&session_id, "user", &user_request))?;

    // ── Agentic loop ──────────────────────────────────────────────────────
    let temperature = storage::get_config("temperature")?
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.3); // lower temp for code generation
    let max_tokens = storage::get_config("max_tokens")?
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(8192); // generous default for code generation

    let mut iteration = 0;
    let mut _last_tool_calls: Vec<StreamedToolCall> = Vec::new();

    loop {
        iteration += 1;
        if iteration > MAX_AGENT_ITERATIONS {
            ui::print_warning(&format!(
                "Reached max {} iterations. Stopping.",
                MAX_AGENT_ITERATIONS
            ));
            break;
        }

        // Build message context — compress first if needed
        let _ = context::maybe_compress(&session_id, &llm).await;
        let messages = context::build_messages(&session_id, &system).await?;

        if iteration > 1 {
            println!();
            println!(
                "  {} {}",
                "↺".bright_cyan().bold(),
                format!("Iteration {} — continuing…", iteration).truecolor(160, 160, 200),
            );
            println!();
        }

        // ── Stream live tokens ────────────────────────────────────────────
        let stream_result = llm
            .chat_stream_live(messages.clone(), temperature, max_tokens)
            .await;

        let (response, streamed_calls) = match stream_result {
            Ok(r) => r,
            Err(e) => {
                let msg = e.to_string();
                // Context size exceeded — force compress and retry once
                if msg.contains("exceed_context_size") || msg.contains("context size") || msg.contains("400") {
                    ui::print_warning("Context too large — compressing and retrying…");
                    // Force compress by temporarily lowering the budget
                    let compressed = force_compress_context(&session_id, &llm).await;
                    if compressed {
                        let messages2 = context::build_messages(&session_id, &system).await?;
                        llm.chat_stream_live(messages2, temperature, max_tokens).await?
                    } else {
                        return Err(e);
                    }
                } else {
                    return Err(e);
                }
            }
        };

        // Store assistant response
        storage::append_message(&Message::new(&session_id, "assistant", &response))?;

        // ── Collect tool calls: streamed (live parsed) + post-parse fallback ─
        // Use the streamed ones first; fall back to post-scan if none found
        let mut tool_calls_this_turn: Vec<(String, serde_json::Value)> = streamed_calls
            .iter()
            .map(|tc| (tc.tool.clone(), tc.args.clone()))
            .collect();

        // Also scan the full response for any tool calls that slipped through
        // (e.g. if the fence was split at a chunk boundary in a weird way)
        if tool_calls_this_turn.is_empty() {
            tool_calls_this_turn = mcp::extract_all_tool_calls(&response);
        }

        // Also run the plain-markdown extractor (for files written without tool_call format)
        let extracted = crate::extractor::extract(&response);

        // Execute explicit tool calls
        let mut any_tool_ran = false;
        if !tool_calls_this_turn.is_empty() {
            println!("  {}", "── Tool calls ──────────────────────────────────────────".truecolor(60, 60, 80));
        }
        for (tool_name, tool_args) in &tool_calls_this_turn {
            any_tool_ran = true;
            let result = mcp::call_tool(tool_name, tool_args, &permissions, true).await;
            storage::append_message(&Message::new(
                &session_id,
                "tool",
                &format!("[tool:{}]\n{}", result.tool_name, result.output),
            ))?;
        }

        // Execute fallback file writes from plain markdown (deduplicate)
        for file in &extracted.files {
            let already = tool_calls_this_turn.iter().any(|(name, args)| {
                name == "write_file" && args["path"].as_str() == Some(&file.path)
            });
            if already { continue; }
            let file_args = serde_json::json!({ "path": file.path, "content": file.content });
            let result = mcp::call_tool("write_file", &file_args, &permissions, true).await;
            if result.success { any_tool_ran = true; }
            storage::append_message(&Message::new(
                &session_id,
                "tool",
                &format!("[tool:write_file]\n{}", result.output),
            ))?;
        }

        // Execute shell commands from plain markdown
        for shell_cmd in &extracted.shell {
            let shell_args = serde_json::json!({ "command": shell_cmd.command, "cwd": workspace_path });
            let result = mcp::call_tool("run_shell", &shell_args, &permissions, true).await;
            any_tool_ran = true;
            storage::append_message(&Message::new(
                &session_id,
                "tool",
                &format!("[tool:run_shell]\n{}", result.output),
            ))?;
        }

        _last_tool_calls = streamed_calls;

        // ── Loop control: if no tool ran, the LLM is done ─────────────────
        if !any_tool_ran {
            break;
        }

        // Let the LLM know we executed everything and it should continue
        let continue_msg = "[System: All tool calls have been executed. \
            Continue building — write any remaining files, run setup commands, \
            and finish the implementation. When fully done, output DONE.]";
        storage::append_message(&Message::new(&session_id, "user", continue_msg))?;

        // Check if the LLM signalled it's done in the response
        if response.contains("DONE") || response.to_lowercase().contains("all done")
            || response.to_lowercase().contains("implementation is complete")
            || response.to_lowercase().contains("project is complete")
        {
            break;
        }
    }

    // ── Final summary ─────────────────────────────────────────────────────
    println!();
    println!("  {}", "─".repeat(70).truecolor(50, 50, 65));
    print_build_summary(workspace_path);
    println!();

    let _ = context::maybe_compress(&session_id, &llm).await;
    Ok(())
}

// ─── Build the user request string ───────────────────────────────────────────

fn build_user_request(args: &BuildArgs, workspace_path: &str) -> Result<String> {
    // If a --from file is given, read it
    if let Some(doc_path) = &args.from {
        let path = std::path::Path::new(doc_path);
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::path::Path::new(workspace_path).join(path)
        };
        let content = std::fs::read_to_string(&abs_path)
            .map_err(|e| anyhow::anyhow!("Cannot read '{}': {}", abs_path.display(), e))?;

        let preamble = if let Some(extra) = &args.prompt {
            format!("Additional instructions: {}\n\n", extra)
        } else {
            String::new()
        };

        return Ok(format!(
            "{}Build the web application described below. Implement EVERYTHING. \
            Use ABSOLUTE paths starting with {workspace_path}/ for every file. \
            NEVER use npx create-react-app or vite scaffolding — write all files yourself.\n\n\
            Requirements document:\n\n{}",
            preamble,
            content,
            workspace_path = workspace_path,
        ));
    }

    // Plain prompt
    let prompt = args.prompt.as_deref().unwrap_or("");
    Ok(format!(
        "Build this web application from scratch: {prompt}\n\n\
        RULES:\n\
        - Use ABSOLUTE paths: every path must start with {workspace_path}/\n\
        - NEVER use npx create-react-app or vite scaffolding — write every file yourself\n\
        - Write package.json FIRST, then run npm install, then write source files\n\
        - Use plain HTML/CSS/JS unless a framework was explicitly requested\n\
        - Every file must be complete and working — no placeholders\n\
        - Output DONE when finished",
        prompt = prompt,
        workspace_path = workspace_path,
    ))
}

// ─── Web-app-builder system prompt ───────────────────────────────────────────

fn build_system_prompt(workspace_path: &str, ws_info: &workspace::WorkspaceInfo) -> String {
    // Describe the existing workspace state
    let ws_state = match &ws_info.kind {
        workspace::WorkspaceKind::ExistingProject => {
            let stack = if ws_info.detected_stack.is_empty() {
                "unknown".to_string()
            } else {
                ws_info.detected_stack.join(", ")
            };
            format!(
                "EXISTING PROJECT (stack: {}). Read existing files before editing. Extend rather than replace.",
                stack
            )
        }
        workspace::WorkspaceKind::RequirementsDoc { doc_path } => {
            format!("A requirements document exists at `{}`. Follow it precisely.", doc_path)
        }
        workspace::WorkspaceKind::Blank => {
            "BLANK DIRECTORY. Build everything from scratch — do NOT use create-react-app or any scaffolding tool. Write every file yourself.".to_string()
        }
    };

    // List existing files if any
    let existing_files = if !ws_info.key_files.is_empty() {
        format!("\nExisting key files: {}", ws_info.key_files.join(", "))
    } else {
        String::new()
    };

    format!(
        r#"You are Shamsu Build — an autonomous web application builder, like Lovable or Bolt.
You build fully working web applications by writing every file yourself using tool calls.

## Workspace
Absolute path: {workspace_path}
State: {ws_state}{existing_files}

## CRITICAL: Order of operations
You MUST follow this exact order every time:
1. Create directories first (create_dir)
2. Write package.json first (if using npm)
3. Run npm install AFTER writing package.json — never before
4. Write all source files AFTER npm install
5. NEVER run `npx create-react-app`, `npm create vite`, or any scaffolding tool — write files yourself
6. NEVER run `npm start` or `npm run dev` — just write files and install deps

## Tool use — MANDATORY
You MUST use tool_call blocks for every file operation. NEVER write code in plain markdown fences.

### Format — one JSON object per block:
```tool_call
{{"tool": "write_file", "args": {{"path": "{workspace_path}/index.html", "content": "<!DOCTYPE html>\n<html>..."}}}}
```

```tool_call
{{"tool": "run_shell", "args": {{"command": "npm install", "cwd": "{workspace_path}"}}}}
```

### IMPORTANT path rules:
- Always use ABSOLUTE paths: start every path with {workspace_path}/
- Example: "{workspace_path}/src/App.js" NOT "src/App.js"
- cwd in run_shell must always be "{workspace_path}"

### Available tools:
- write_file(path, content) — create or overwrite a file (use absolute path)
- read_file(path) — read a file
- patch_file(path, old_str, new_str) — targeted find-and-replace in a file
- delete_file(path) — delete a file
- create_dir(path) — create a directory tree
- run_shell(command, cwd) — run a shell command (always set cwd to {workspace_path})
- list_dir(path) — list directory contents
- search_files(pattern, directory) — recursive text search

### Escaping rules for content strings:
- Newlines → \n
- Double quotes inside content → \"
- Backslashes → \\

## Technology choices
- DEFAULT (no framework requested): plain HTML + CSS + JS in a single index.html or split files
- React requested: write package.json + src/index.jsx + src/App.jsx manually, then npm install react react-dom; use esbuild or vite as bundler
- Always write a complete package.json with "scripts": {{"dev": "...", "build": "..."}}
- Use localStorage for all client-side persistence
- Use CSS variables for theming, flexbox/grid for layout
- Include dark mode via prefers-color-scheme or a toggle
- Write mobile-responsive CSS

## Code quality rules
- Write COMPLETE code — zero placeholders, zero `// TODO`
- Every function must be implemented
- Include basic error handling
- Comment non-obvious logic

## Completion
After writing ALL files and running all setup commands:
1. Write a one-paragraph summary of what was built
2. List the files created
3. Say exactly how to run it
4. Output the word DONE on its own line
"#,
        workspace_path = workspace_path,
        ws_state = ws_state,
        existing_files = existing_files,
    )
}

// ─── Print build summary ──────────────────────────────────────────────────────

fn print_build_summary(workspace_path: &str) {
    // Count files written
    let count = count_workspace_files(workspace_path);

    println!(
        "  {} {}",
        "✓ Build complete!".bright_green().bold(),
        format!("({} files in {})", count, workspace_path).truecolor(160, 160, 200),
    );

    // Detect if there's a package.json with a dev/start script
    let pkg_path = std::path::Path::new(workspace_path).join("package.json");
    if pkg_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_path) {
            if content.contains("\"dev\"") || content.contains("\"start\"") {
                println!();
                println!("  {} To start the dev server:", "▶".bright_cyan().bold());
                if content.contains("\"dev\"") {
                    println!("    {} {}", "$".truecolor(100, 200, 100), format!("cd {} && npm run dev", workspace_path).bright_white().bold());
                } else {
                    println!("    {} {}", "$".truecolor(100, 200, 100), format!("cd {} && npm start", workspace_path).bright_white().bold());
                }
            }
        }
    }

    // Detect index.html for static sites
    let index_path = std::path::Path::new(workspace_path).join("index.html");
    if index_path.exists() && !pkg_path.exists() {
        println!();
        println!("  {} Static site ready. Open:", "▶".bright_cyan().bold());
        println!("    {}", index_path.to_string_lossy().bright_white().bold());
    }
}

fn count_workspace_files(workspace_path: &str) -> usize {
    let mut count = 0;
    let skip_dirs = ["node_modules", ".git", "target", "dist", ".next"];
    if let Ok(entries) = std::fs::read_dir(workspace_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if skip_dirs.contains(&name.as_str()) { continue; }
            if entry.path().is_file() {
                count += 1;
            } else if entry.path().is_dir() {
                count += count_workspace_files(&entry.path().to_string_lossy());
            }
        }
    }
    count
}

/// Force-compress context by summarising ALL non-system messages, not just half.
/// Used as a recovery mechanism when a 400 context-exceeded error is returned.
/// Returns true if compression happened, false if there was nothing to compress.
async fn force_compress_context(session_id: &str, llm: &LlmClient) -> bool {
    let messages = match storage::get_messages(session_id, None) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if messages.len() < 2 {
        return false;
    }

    // Summarise all but the last 2 messages (keep the most recent exchange)
    let keep = 2;
    let to_compress = &messages[..messages.len().saturating_sub(keep)];
    if to_compress.is_empty() {
        return false;
    }

    let last_id = match to_compress.last().and_then(|m| m.id) {
        Some(id) => id,
        None => return false,
    };

    let conversation_text: String = to_compress
        .iter()
        .map(|m| format!("[{}]: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let summary_request = vec![
        crate::llm::ChatMessage {
            role: "system".to_string(),
            content: "You are a conversation summarizer. Produce a concise but complete \
                      summary preserving all file paths created, commands run, tool results, \
                      decisions made, and current build state. Be terse but complete.".to_string(),
        },
        crate::llm::ChatMessage {
            role: "user".to_string(),
            content: conversation_text,
        },
    ];

    ui::print_info("Summarising conversation history to free context…");
    match llm.chat(summary_request, 0.2, 1024).await {
        Ok(summary) => {
            let _ = storage::save_snapshot(session_id, &summary, last_id);
            let _ = storage::archive_messages_before(session_id, last_id);
            ui::print_success("Context compressed.");
            true
        }
        Err(_) => false,
    }
}
