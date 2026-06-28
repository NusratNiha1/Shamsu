/// commands/chat.rs — Agentic chat loop with Claude Code-style UI.
///
/// Flow per turn:
///   1. User types → stored in DB
///   2. Build layered context (system / memory / messages)
///   3. Stream LLM response
///   4. Extract ALL tool calls from response
///   5. For each tool call: show card → prompt permission → execute → feed result back
///   6. Loop until no more tool calls (max 12 iterations)
///   7. Compress context if needed

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use rustyline::{DefaultEditor, error::ReadlineError};

use crate::context;
use crate::extractor;
use crate::llm::LlmClient;
use crate::mcp;
use crate::permissions::Permissions;
use crate::skills;
use crate::storage::{self, Message, Session};
use crate::ui;
use crate::workspace;

const MAX_TOOL_ITERATIONS: usize = 12; // kept for slash-command loop guard

#[derive(Args)]
pub struct ChatArgs {
    /// Session name (auto-creates if missing)
    #[arg(short, long)]
    pub session: Option<String>,

    /// Permission profile: safe | standard | full
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Dry-run: show what would happen without writing anything
    #[arg(long)]
    pub dry_run: bool,

    /// Single message mode — send one message and exit
    #[arg(short, long)]
    pub message: Option<String>,

    /// Skip LLM server check at startup
    #[arg(long)]
    pub no_check: bool,
}

pub async fn run(args: ChatArgs, workspace_path: &str, auto_yes: bool, verbose: bool) -> Result<()> {
    // ── 1. Server check ───────────────────────────────────────────────────
    let llm = LlmClient::new();
    if !args.no_check && !llm.is_alive().await {
        ui::print_error(&format!(
            "Cannot reach llama.cpp server at {}\nRun `shamsu status` for help.",
            crate::llm::llm_url()
        ));
        return Ok(());
    }

    // ── 2. Seed skills ────────────────────────────────────────────────────
    let _ = skills::seed_builtin_skills();

    // ── 3. Session ────────────────────────────────────────────────────────
    let session = resolve_session(&args.session, workspace_path, &args.profile).await?;
    let session_id = session.id.clone();
    let profile_str = args.profile.as_deref().unwrap_or(&session.permission_profile);
    let permissions = Permissions::new(profile_str, workspace_path, args.dry_run);

    // ── 4. Workspace ──────────────────────────────────────────────────────
    let ws_info = workspace::detect(workspace_path).await?;

    // ── 5. Banner + header ────────────────────────────────────────────────
    ui::print_banner();
    let model = llm.model_name().await.unwrap_or_else(|_| "offline".into());
    ui::print_session_header(
        &session.name,
        &ws_info.kind.to_string(),
        profile_str,
        &model,
        &session.active_skills,
    );

    // ── 6. System prompt ──────────────────────────────────────────────────
    let skills_prompt = skills::build_skills_prompt(&session.active_skills);
    let ws_ctx = workspace::context_description(&ws_info);
    let system = context::build_system_prompt(&ws_ctx, &skills_prompt, profile_str);

    // ── 7. Single-message mode ────────────────────────────────────────────
    if let Some(msg) = args.message {
        return run_turn(&msg, &session_id, &system, &llm, &permissions, auto_yes, verbose).await;
    }

    // ── 8. REPL ───────────────────────────────────────────────────────────
    let mut rl = DefaultEditor::new()?;
    let history_path = crate::storage::shamsu_dir().join("history.txt");
    let _ = rl.load_history(&history_path);

    loop {
        // Prompt — styled like Claude Code
        let prompt = format!(
            "{} {} ",
            "❯".truecolor(100, 160, 255).bold(),
            workspace_path.truecolor(80, 80, 100),
        );

        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim().to_string();
                if input.is_empty() { continue; }
                let _ = rl.add_history_entry(&input);

                if input.starts_with('/') {
                    if handle_slash(&input, &session_id, &ws_info, &permissions, &llm, &session).await? {
                        break; // /exit
                    }
                    continue;
                }

                ui::separator();
                if let Err(e) = run_turn(&input, &session_id, &system, &llm, &permissions, auto_yes, verbose).await {
                    ui::print_error(&e.to_string());
                }
                ui::separator();
            }
            Err(ReadlineError::Interrupted) => {
                println!("  {}", "(Ctrl+C — type /exit to quit)".truecolor(100, 100, 120));
            }
            Err(ReadlineError::Eof) => {
                println!("  {}", "Goodbye.".dimmed());
                break;
            }
            Err(e) => {
                ui::print_error(&format!("Readline: {e}"));
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
    Ok(())
}

// ─── Core agentic turn ────────────────────────────────────────────────────────

async fn run_turn(
    user_input: &str,
    session_id: &str,
    system: &str,
    llm: &LlmClient,
    permissions: &Permissions,
    auto_yes: bool,
    verbose: bool,
) -> Result<()> {
    // Store user message
    storage::append_message(&Message::new(session_id, "user", user_input))?;

    let temperature = crate::storage::get_config("temperature")?
        .and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.7);
    let max_tokens = crate::storage::get_config("max_tokens")?
        .and_then(|v| v.parse::<i32>().ok()).unwrap_or(2048);
    let use_stream = crate::storage::get_config("stream")?
        .map(|v| v != "false").unwrap_or(true);

    let messages = context::build_messages(session_id, system).await?;

    if verbose {
        ui::print_info(&format!("{} messages in context", messages.len()));
    }

    // ── Call LLM ──────────────────────────────────────────────────────────
    let response = if use_stream {
        llm.chat_stream(messages, temperature, max_tokens).await?
    } else {
        let r = llm.chat(messages, temperature, max_tokens).await?;
        println!("\n  {} {}\n", "◆ Shamsu".bright_cyan().bold(), r);
        r
    };

    // Store the assistant message
    storage::append_message(&Message::new(session_id, "assistant", &response))?;

    // ── Step 1: handle explicit tool_call blocks (model did the right thing) ──
    let tool_calls = mcp::extract_all_tool_calls(&response);

    if !tool_calls.is_empty() {
        println!();
        for (tool_name, tool_args) in &tool_calls {
            let result = mcp::call_tool(tool_name, tool_args, permissions, auto_yes).await;
            storage::append_message(&Message::new(
                session_id,
                "tool",
                &format!("Tool `{}` result:\n{}", result.tool_name, result.output),
            ))?;
        }
    }

    // ── Step 2: extract code blocks and shell commands from plain markdown ──
    let extracted = extractor::extract(&response);

    // Write files
    let mut written_files: Vec<String> = Vec::new();
    for file in &extracted.files {
        let already_written = tool_calls.iter().any(|(name, args)| {
            name == "write_file" && args["path"].as_str() == Some(&file.path)
        });
        if already_written { continue; }

        let args = serde_json::json!({
            "path": file.path,
            "content": file.content
        });
        let result = mcp::call_tool("write_file", &args, permissions, auto_yes).await;
        if result.success { written_files.push(file.path.clone()); }
        storage::append_message(&Message::new(
            session_id,
            "tool",
            &format!("Tool `write_file` result:\n{}", result.output),
        ))?;
    }

    // Execute shell commands (only if profile allows)
    for shell_cmd in &extracted.shell {
        if permissions.can_execute_shell().is_err() {
            ui::print_warning(&format!(
                "Shell skipped (profile: {}). Use --profile full: {}",
                permissions.profile.as_str(),
                shell_cmd.command
            ));
            continue;
        }
        let args = serde_json::json!({ "command": shell_cmd.command });
        let result = mcp::call_tool("run_shell", &args, permissions, auto_yes).await;
        storage::append_message(&Message::new(
            session_id,
            "tool",
            &format!("Tool `run_shell` result:\n{}", result.output),
        ))?;
    }

    // ── Summary / plain-text answer ───────────────────────────────────────
    let total_written = written_files.len()
        + tool_calls.iter().filter(|(n, _)| n == "write_file").count();

    println!();
    if total_written > 0 {
        let names: Vec<String> = written_files.iter()
            .chain(
                tool_calls.iter()
                    .filter(|(n, _)| n == "write_file")
                    .filter_map(|(_, a)| a["path"].as_str().map(String::from))
                    .collect::<Vec<_>>()
                    .iter()
            )
            .cloned()
            .collect();
        ui::print_success(&format!("Done — {} file(s) written: {}", total_written, names.join(", ")));
    } else if extracted.files.is_empty() && extracted.shell.is_empty() && tool_calls.is_empty() {
        // Pure conversational answer — print the response text
        println!("  {}", "◆ Shamsu".bright_cyan().bold());
        for line in response.lines() {
            println!("  {}", line);
        }
    }
    println!();

    let _ = context::maybe_compress(session_id, llm).await;
    Ok(())
}

// ─── Slash command handler — returns true if /exit ───────────────────────────

async fn handle_slash(
    input: &str,
    session_id: &str,
    ws_info: &workspace::WorkspaceInfo,
    permissions: &Permissions,
    llm: &LlmClient,
    _session: &Session,
) -> Result<bool> {
    // Support short aliases too
    let cmd = input.split_whitespace().next().unwrap_or(input);
    match cmd {
        "/exit" | "/quit" | "/q" => {
            println!("  {}", "Goodbye.".truecolor(120, 120, 140));
            return Ok(true);
        }
        "/help" | "/h" => ui::print_help(),
        "/clear" | "/cl" => {
            let conn = storage::open()?;
            conn.execute(
                "UPDATE messages SET is_archived = 1 WHERE session_id = ?1",
                rusqlite::params![session_id],
            )?;
            ui::print_success("Context cleared (history preserved on disk).");
        }
        "/compact" | "/co" => {
            ui::print_info("Compressing context…");
            let _ = context::maybe_compress(session_id, llm).await;
            ui::print_success("Done.");
        }
        "/skills" | "/sk" => {
            let all = crate::skills::Skill::list_all()?;
            crate::skills::print_skill_list(&all);
        }
        "/profile" | "/pr" => permissions.print_summary(),
        "/status" | "/st" => crate::commands::status::check().await?,
        "/inspect" | "/in" => workspace::print_info(ws_info),
        "/undo" | "/u" => {
            // Show the last tool message (what was last done)
            let msgs = storage::get_messages(session_id, Some(20))?;
            let last_tool = msgs.iter().rev().find(|m| m.role == "tool");
            match last_tool {
                Some(m) => {
                    ui::print_info("Last tool result:");
                    println!("  {}", m.content.truecolor(180, 180, 200));
                }
                None => ui::print_info("No tool actions in this context window."),
            }
        }
        _ => {
            ui::print_warning(&format!("Unknown command '{}'. Type /help.", input));
        }
    }
    Ok(false)
}

// ─── Session resolution ───────────────────────────────────────────────────────

async fn resolve_session(
    name_opt: &Option<String>,
    workspace_path: &str,
    profile_opt: &Option<String>,
) -> Result<Session> {
    let profile = profile_opt.as_deref().unwrap_or("standard");

    match name_opt {
        Some(name) => {
            if let Some(s) = storage::get_session_by_name(name)? {
                storage::set_active_session(&s.id)?;
                Ok(s)
            } else {
                let mut s = Session::new(name, workspace_path);
                s.permission_profile = profile.into();
                storage::create_session(&s)?;
                storage::set_active_session(&s.id)?;
                ui::print_success(&format!("Created session '{}' ({})", name, profile));
                Ok(s)
            }
        }
        None => {
            if let Some(s) = storage::get_active_session()? {
                Ok(s)
            } else {
                let mut s = Session::new("default", workspace_path);
                s.permission_profile = profile.into();
                storage::create_session(&s)?;
                storage::set_active_session(&s.id)?;
                ui::print_success(&format!("Created default session ({})", profile));
                Ok(s)
            }
        }
    }
}
