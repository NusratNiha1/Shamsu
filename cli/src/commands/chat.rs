/// Interactive chat command — the heart of Shamsu.
///
/// Flow:
///   1. Resolve or auto-create the active session
///   2. Detect workspace (kind, stack, git)
///   3. Load active skills + build system prompt
///   4. Enter readline loop
///   5. For each user message:
///      a. Append to DB
///      b. Build layered context (system / memory / active)
///      c. Stream response from llama.cpp
///      d. Parse any tool calls, execute them, re-send results
///      e. Append assistant turn to DB
///      f. Maybe compress old context

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use rustyline::{DefaultEditor, error::ReadlineError};

use crate::context;
use crate::llm::LlmClient;
use crate::mcp;
use crate::permissions::Permissions;
use crate::skills;
use crate::storage::{self, Message, Session};
use crate::workspace;

/// How many tool-call loops we allow before giving up to avoid infinite recursion
const MAX_TOOL_ITERATIONS: usize = 6;

#[derive(Args)]
pub struct ChatArgs {
    /// Attach to a specific session by name (auto-creates if missing)
    #[arg(short, long)]
    pub session: Option<String>,

    /// Permission profile override: safe | standard | full
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Enable dry-run mode (no file writes or shell execution)
    #[arg(long)]
    pub dry_run: bool,

    /// Send a single message and exit (non-interactive)
    #[arg(short, long)]
    pub message: Option<String>,

    /// Skip the LLM server check at startup
    #[arg(long)]
    pub no_check: bool,
}

pub async fn run(
    args: ChatArgs,
    workspace_path: &str,
    _auto_yes: bool,
    verbose: bool,
) -> Result<()> {
    // ── 1. LLM reachability check ──────────────────────────────────────────
    let llm = LlmClient::new();
    if !args.no_check {
        if !llm.is_alive().await {
            println!(
                "\n  {} Cannot reach llama.cpp server at {}",
                "✗".bright_red().bold(),
                crate::llm::llm_url().dimmed()
            );
            println!(
                "  {}",
                "Run `shamsu status` for setup instructions.".yellow()
            );
            println!(
                "  {}",
                "Or start chat anyway with --no-check (responses will fail until server is up).".dimmed()
            );
            return Ok(());
        }
    }

    // ── 2. Seed built-in skills once ──────────────────────────────────────
    let _ = skills::seed_builtin_skills();

    // ── 3. Resolve session ────────────────────────────────────────────────
    let session = resolve_session(&args.session, workspace_path, &args.profile).await?;
    let session_id = session.id.clone();

    // Permission profile: arg > session > default
    let profile_str = args
        .profile
        .as_deref()
        .unwrap_or(&session.permission_profile);
    let permissions = Permissions::new(profile_str, workspace_path, args.dry_run);

    // ── 4. Workspace detection ────────────────────────────────────────────
    let ws_info = workspace::detect(workspace_path).await?;

    // ── 5. Print session header ───────────────────────────────────────────
    print_chat_header(&session, &ws_info, &permissions, verbose);

    // ── 6. Build tool definitions list for system prompt ─────────────────
    let tools = mcp::builtin_tool_defs();

    // ── 7. Build base system prompt ───────────────────────────────────────
    let skills_prompt = skills::build_skills_prompt(&session.active_skills);
    let ws_ctx = workspace::context_description(&ws_info);
    let base_system = context::build_system_prompt(&ws_ctx, &skills_prompt, profile_str);

    // Append tool definitions so the LLM knows what tools are available
    let system_with_tools = format!(
        "{}{}",
        base_system,
        mcp::tools_system_prompt(&tools)
    );

    // ── 8. Single-message mode ────────────────────────────────────────────
    if let Some(msg) = args.message {
        return run_single(
            &msg,
            &session_id,
            &system_with_tools,
            &llm,
            &permissions,
            verbose,
        )
        .await;
    }

    // ── 9. Interactive REPL loop ──────────────────────────────────────────
    let mut rl = DefaultEditor::new()?;

    // Load previous history from file if it exists
    let history_path = crate::storage::shamsu_dir().join("history.txt");
    let _ = rl.load_history(&history_path);

    println!(
        "  {}",
        "Type your message. Commands: /help  /clear  /skills  /profile  /exit".dimmed()
    );
    println!();

    loop {
        let readline = rl.readline(&format!("{} ", "You:".bright_yellow().bold()));
        match readline {
            Ok(line) => {
                let input = line.trim().to_string();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(&input);

                // ── Slash commands ──────────────────────────────────────
                if input.starts_with('/') {
                    match input.as_str() {
                        "/exit" | "/quit" | "/q" => {
                            println!("  {}", "Goodbye.".dimmed());
                            break;
                        }
                        "/help" => {
                            print_help();
                            continue;
                        }
                        "/clear" => {
                            // Archive all messages so they don't appear but are not deleted
                            let conn = storage::open()?;
                            conn.execute(
                                "UPDATE messages SET is_archived = 1 WHERE session_id = ?1",
                                rusqlite::params![session_id],
                            )?;
                            println!("  {} Context cleared.", "✓".bright_green());
                            continue;
                        }
                        "/skills" => {
                            let all = crate::skills::Skill::list_all()?;
                            crate::skills::print_skill_list(&all);
                            continue;
                        }
                        "/profile" => {
                            permissions.print_summary();
                            continue;
                        }
                        "/status" => {
                            crate::commands::status::check().await?;
                            continue;
                        }
                        "/inspect" => {
                            workspace::print_info(&ws_info);
                            continue;
                        }
                        _ => {
                            if let Some(rest) = input.strip_prefix("/profile ") {
                                println!(
                                    "  {} Profile changes require starting a new chat session.",
                                    "i".bright_blue()
                                );
                                println!(
                                    "  Use: {} to change the session profile, then restart.",
                                    format!("shamsu session set-profile {} {}", session.name, rest)
                                        .bright_white()
                                );
                            } else {
                                println!(
                                    "  {} Unknown command '{}'. Type /help for commands.",
                                    "?".yellow(),
                                    input
                                );
                            }
                            continue;
                        }
                    }
                }

                // ── Chat turn ───────────────────────────────────────────
                let result = run_chat_turn(
                    &input,
                    &session_id,
                    &system_with_tools,
                    &llm,
                    &permissions,
                    verbose,
                )
                .await;

                if let Err(e) = result {
                    println!("\n  {} {}", "Error:".bright_red().bold(), e);
                    if verbose {
                        println!("{:?}", e);
                    }
                }
            }

            Err(ReadlineError::Interrupted) => {
                // Ctrl-C — just continue
                println!("  {}", "(Use /exit to quit)".dimmed());
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                println!("  {}", "Goodbye.".dimmed());
                break;
            }
            Err(e) => {
                println!("  {} Readline error: {}", "✗".bright_red(), e);
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_path);

    Ok(())
}

// ─── Single message (non-interactive) ─────────────────────────────────────────

async fn run_single(
    message: &str,
    session_id: &str,
    system_prompt: &str,
    llm: &LlmClient,
    permissions: &Permissions,
    verbose: bool,
) -> Result<()> {
    run_chat_turn(message, session_id, system_prompt, llm, permissions, verbose).await
}

// ─── Core chat turn ────────────────────────────────────────────────────────────

/// Run one complete user→assistant exchange, including any tool call loops.
async fn run_chat_turn(
    user_input: &str,
    session_id: &str,
    system_prompt: &str,
    llm: &LlmClient,
    permissions: &Permissions,
    verbose: bool,
) -> Result<()> {
    // Store user message
    let user_msg = Message::new(session_id, "user", user_input);
    storage::append_message(&user_msg)?;

    let temperature = crate::storage::get_config("temperature")?
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.7);

    let max_tokens = crate::storage::get_config("max_tokens")?
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(2048);

    let use_stream = crate::storage::get_config("stream")?
        .map(|v| v != "false")
        .unwrap_or(true);

    let mut iteration = 0;

    loop {
        if iteration >= MAX_TOOL_ITERATIONS {
            println!(
                "\n  {} Reached tool-call limit ({}).",
                "⚠".yellow(),
                MAX_TOOL_ITERATIONS
            );
            break;
        }

        // Build message list with context layers
        let messages = context::build_messages(session_id, system_prompt).await?;

        if verbose {
            println!(
                "  {} {} messages in context",
                "ctx:".dimmed(),
                messages.len()
            );
        }

        // Call LLM
        let response = if use_stream {
            llm.chat_stream(messages, temperature, max_tokens).await?
        } else {
            let r = llm.chat(messages, temperature, max_tokens).await?;
            println!("{} {}", "Shamsu:".bright_green().bold(), r);
            r
        };

        // Check for tool call in response
        if let Some((tool_name, tool_args)) = mcp::extract_tool_call(&response) {
            if verbose {
                println!(
                    "  {} calling {} with {}",
                    "tool:".bright_magenta(),
                    tool_name.bold(),
                    tool_args
                );
            }

            // Store assistant's message (which contains the tool call)
            let assistant_msg = Message::new(session_id, "assistant", &response);
            storage::append_message(&assistant_msg)?;

            // Execute the tool
            let result = mcp::call_tool(&tool_name, &tool_args, permissions).await;

            // Feed tool result back as a "tool" role message
            let tool_msg = Message::new(
                session_id,
                "tool",
                &format!(
                    "Tool `{}` result:\n{}",
                    result.tool_name, result.output
                ),
            );
            storage::append_message(&tool_msg)?;

            iteration += 1;
            // Loop: LLM will see the tool result and (hopefully) produce a final answer
            continue;
        }

        // No tool call — this is the final response
        let assistant_msg = Message::new(session_id, "assistant", &response);
        storage::append_message(&assistant_msg)?;

        // Maybe compress old context
        let _ = context::maybe_compress(session_id, llm).await;

        break;
    }

    Ok(())
}

// ─── Session resolution ────────────────────────────────────────────────────────

/// Get or create the active session.
async fn resolve_session(
    name_opt: &Option<String>,
    workspace_path: &str,
    profile_opt: &Option<String>,
) -> Result<Session> {
    match name_opt {
        Some(name) => {
            // Named session: get or create
            if let Some(s) = storage::get_session_by_name(name)? {
                storage::set_active_session(&s.id)?;
                Ok(s)
            } else {
                // Auto-create
                let profile = profile_opt.as_deref().unwrap_or("standard");
                let mut session = Session::new(name, workspace_path);
                session.permission_profile = profile.to_string();
                storage::create_session(&session)?;
                storage::set_active_session(&session.id)?;
                println!(
                    "  {} Created session {} ({})",
                    "✓".bright_green(),
                    name.bold(),
                    profile.bright_yellow()
                );
                Ok(session)
            }
        }
        None => {
            // No name given — use or create a default session
            if let Some(s) = storage::get_active_session()? {
                Ok(s)
            } else {
                let profile = profile_opt.as_deref().unwrap_or("standard");
                let mut session = Session::new("default", workspace_path);
                session.permission_profile = profile.to_string();
                storage::create_session(&session)?;
                storage::set_active_session(&session.id)?;
                println!(
                    "  {} Created default session ({})",
                    "✓".bright_green(),
                    profile.bright_yellow()
                );
                Ok(session)
            }
        }
    }
}

// ─── UI helpers ───────────────────────────────────────────────────────────────

fn print_chat_header(
    session: &Session,
    ws_info: &workspace::WorkspaceInfo,
    permissions: &Permissions,
    verbose: bool,
) {
    println!(
        "\n{}",
        "── Shamsu Chat ────────────────────────────────────".dimmed()
    );
    println!(
        "  {} {}",
        "Session:".bold(),
        session.name.bright_cyan()
    );
    println!(
        "  {} {}",
        "Workspace:".bold(),
        ws_info.kind.to_string().bright_yellow()
    );
    permissions.print_summary();

    if !session.active_skills.is_empty() {
        println!(
            "  {} {}",
            "Skills:".bold(),
            session.active_skills.join(", ").bright_magenta()
        );
    }

    if verbose && !ws_info.detected_stack.is_empty() {
        println!(
            "  {} {}",
            "Stack:".bold(),
            ws_info.detected_stack.join(", ").dimmed()
        );
    }

    println!(
        "{}",
        "────────────────────────────────────────────────────".dimmed()
    );
    println!();
}

fn print_help() {
    println!(
        "\n{}",
        "── Commands ──────────────────────────────────────".dimmed()
    );
    let cmds = [
        ("/exit, /quit, /q", "Exit the chat"),
        ("/clear", "Clear active context (archived, not deleted)"),
        ("/skills", "List available skills"),
        ("/profile", "Show active permission profile"),
        ("/status", "Check llama.cpp server status"),
        ("/inspect", "Show workspace detection info"),
        ("/help", "Show this help"),
    ];
    for (cmd, desc) in &cmds {
        println!("  {}  {}", cmd.bright_cyan().bold(), desc.dimmed());
    }
    println!(
        "{}",
        "──────────────────────────────────────────────────".dimmed()
    );
    println!();
}
