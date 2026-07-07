/// Context window manager.
///
/// Shamsu uses a layered context:
///   1. System layer  — identity, skills, permissions (always present)
///   2. Workspace layer — project structure, key files
///   3. Memory layer  — compressed summaries of old conversation turns
///   4. Active layer  — last N messages + currently open files
///
/// When the active layer exceeds MAX_ACTIVE_TOKENS, the oldest half is
/// summarised by the LLM and stored as a memory snapshot. The raw messages
/// are archived in SQLite (not deleted).

use anyhow::Result;
use crate::llm::{ChatMessage, LlmClient};
use crate::storage;

/// Rough token budget for the active (non-archived) window.
/// With 32K context on the server we can afford a much larger window.
/// We stay conservative at 20K to leave headroom for the response.
const MAX_ACTIVE_TOKENS: usize = 20_000;

/// Very rough approximation: 1 token ≈ 4 characters
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Build the full message list to send to the LLM.
///
/// Order:
///   system_message
///   [memory_snapshot if present]
///   recent messages (active layer)
pub async fn build_messages(
    session_id: &str,
    system_prompt: &str,
) -> Result<Vec<ChatMessage>> {
    let mut messages: Vec<ChatMessage> = Vec::new();

    // 1. System prompt
    messages.push(ChatMessage {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    });

    // 2. Memory snapshot (compressed history)
    if let Ok(Some(summary)) = storage::get_latest_snapshot(session_id) {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!(
                "[Conversation summary so far]\n{}",
                summary
            ),
        });
    }

    // 3. Recent (active) messages
    let recent = storage::get_messages(session_id, Some(60))?;
    for msg in &recent {
        messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    Ok(messages)
}

/// After receiving a response, check if we need to compress old messages.
/// If the total token estimate exceeds the budget, summarise the oldest half.
pub async fn maybe_compress(
    session_id: &str,
    llm: &LlmClient,
) -> Result<()> {
    let messages = storage::get_messages(session_id, None)?;
    let total_tokens: usize = messages.iter().map(|m| estimate_tokens(&m.content)).sum();

    if total_tokens <= MAX_ACTIVE_TOKENS {
        return Ok(()); // nothing to do
    }

    // Summarise the oldest half
    let half = messages.len() / 2;
    if half < 4 {
        return Ok(()); // not enough messages to bother
    }

    let oldest = &messages[..half];
    let last_id = oldest.last().unwrap().id.unwrap_or(0);

    let conversation_text: String = oldest
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let summary_request = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a conversation summarizer. Produce a concise but complete \
                      summary of the following conversation, preserving all important \
                      decisions, code context, and facts. Be terse."
                .to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: conversation_text,
        },
    ];

    let summary = llm.chat(summary_request, 0.3, 1024).await?;

    // Store snapshot and archive the messages
    storage::save_snapshot(session_id, &summary, last_id)?;
    storage::archive_messages_before(session_id, last_id)?;

    Ok(())
}

/// Build the system prompt from workspace info, skills, and permission profile
pub fn build_system_prompt(
    workspace_context: &str,
    skills_prompt: &str,
    permission_profile: &str,
) -> String {
    let core = concat!(
        "You are Shamsu, an offline-first AI developer assistant running locally.\n\n",
        "When writing code, always:\n",
        "- Put each file in its own fenced code block with the language tag\n",
        "- On the line immediately before the code block, write the filename like this:\n",
        "  `index.html`\n",
        "  ```html\n",
        "  ...code...\n",
        "  ```\n",
        "- Include ALL files needed (HTML, CSS, JS separately)\n",
        "- Use localStorage for browser-side persistence\n\n",
    );

    let mut prompt = String::from(core);

    prompt.push_str(&format!("## Workspace\n{}\n\n", workspace_context));

    prompt.push_str(&format!(
        "## Permissions\nActive profile: **{}**. ",
        permission_profile
    ));
    match permission_profile {
        "safe" => prompt.push_str("You may read files but MUST NOT write or execute anything.\n\n"),
        "standard" => prompt.push_str(
            "You may read and write files within the workspace. Shell execution is disabled.\n\n",
        ),
        "full" => prompt.push_str(
            "You may read/write files and execute shell commands. \
             Always show the user what you intend to do before doing it.\n\n",
        ),
        _ => prompt.push_str("\n\n"),
    }

    if !skills_prompt.is_empty() {
        prompt.push_str(skills_prompt);
        prompt.push('\n');
    }

    prompt
}
