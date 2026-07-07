use anyhow::{anyhow, Result};
use colored::Colorize;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

/// A tool call parsed out of a live stream.
#[derive(Debug, Clone)]
pub struct StreamedToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

const DEFAULT_LLM_URL: &str = "http://127.0.0.1:8080";

pub fn llm_url() -> String {
    crate::storage::get_config("llm_url")
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_LLM_URL.to_string())
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: f32,
    top_p: f32,
    max_tokens: i32,
    stop: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo { id: String }

#[derive(Debug, Deserialize)]
struct ModelsResponse { data: Vec<ModelInfo> }

pub struct LlmClient {
    client: Client,
    base_url: String,
}

impl LlmClient {
    pub fn new() -> Self {
        LlmClient {
            client: Client::builder()
                .timeout(Duration::from_secs(600))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: llm_url(),
        }
    }

    pub async fn is_alive(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(4))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn model_name(&self) -> Result<String> {
        let resp = self.client
            .get(format!("{}/v1/models", self.base_url))
            .send().await?
            .json::<ModelsResponse>().await?;
        Ok(resp.data.into_iter().next().map(|m| m.id).unwrap_or_else(|| "unknown".into()))
    }

    /// Stream response to stdout with an animated thinking spinner.
    /// Returns the full response text.
    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: i32,
    ) -> Result<String> {
        let request = ChatCompletionRequest {
            messages,
            stream: true,
            temperature,
            top_p: 0.95,
            max_tokens,
            stop: vec!["<|endoftext|>".into(), "</s>".into()],
        };

        // ── Animated spinner ──────────────────────────────────────────────
        let done_flag = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done_flag);
        let mut spinner: Option<std::thread::JoinHandle<()>> = Some(std::thread::spawn(move || {
            let phases = [
                ("🍳", "Cracking eggs…"),
                ("🔥", "Heating things up…"),
                ("✨", "Conjuring some magic…"),
                ("🧙", "Casting spells on your code…"),
                ("⚗️ ", "Brewing the solution…"),
                ("🍵", "Steeping the logic…"),
                ("🔮", "Gazing into the crystal ball…"),
                ("🎨", "Painting pixels…"),
                ("🧩", "Fitting the pieces…"),
                ("🚀", "Preparing for launch…"),
            ];
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let start = Instant::now();
            let mut tick = 0usize;
            while !done_clone.load(Ordering::Relaxed) {
                let secs = start.elapsed().as_secs();
                let phase_idx = (secs / 4) as usize % phases.len();
                let (icon, msg) = phases[phase_idx];
                let frame = frames[tick % frames.len()];
                print!(
                    "\r  {} {} {}  {}   ",
                    frame.bright_cyan(),
                    icon,
                    msg.bright_magenta().bold(),
                    format!("{}s", secs).truecolor(90, 90, 110),
                );
                let _ = io::stdout().flush();
                std::thread::sleep(Duration::from_millis(80));
                tick += 1;
            }
        }));

        // Inline helper: signal done and join the thread, then clear line.
        macro_rules! stop_spinner {
            ($sp:expr) => {{
                done_flag.store(true, Ordering::Relaxed);
                if let Some(h) = $sp.take() { let _ = h.join(); }
                print!("\r{}\r", " ".repeat(78));
                let _ = io::stdout().flush();
            }};
        }

        // ── HTTP request ──────────────────────────────────────────────────
        let response = match self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                stop_spinner!(spinner);
                return Err(anyhow!(
                    "Cannot reach llama.cpp server at {}.\nError: {}",
                    self.base_url, e
                ));
            }
        };

        if !response.status().is_success() {
            stop_spinner!(spinner);
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("LLM server error {}: {}", status, body));
        }

        // ── Stream tokens silently ────────────────────────────────────────
        // We collect the full response but don't print tokens — the cooking
        // animation is already showing. File writes are shown after this returns.
        let mut stream = response.bytes_stream();
        let mut full = String::new();
        let mut got_tokens = false;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { break; }

                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                    for choice in parsed.choices {
                        if let Some(content) = choice.delta.content {
                            got_tokens = true;
                            full.push_str(&content);
                        }
                        if choice.finish_reason.is_some() { break; }
                    }
                }
            }
        }

        stop_spinner!(spinner);

        if !got_tokens {
            println!("  {}", "(no response)".dimmed());
        }
        // Response is returned to caller — caller decides what to print/do with it.

        Ok(full)
    }

    /// Stream response token-by-token to stdout, printing as they arrive.
    /// Also parses ```tool_call``` blocks on the fly.
    /// Returns (full_response, tool_calls_found).
    pub async fn chat_stream_live(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: i32,
    ) -> Result<(String, Vec<StreamedToolCall>)> {
        let request = ChatCompletionRequest {
            messages,
            stream: true,
            temperature,
            top_p: 0.95,
            max_tokens,
            stop: vec!["<|endoftext|>".into(), "</s>".into()],
        };

        // Print the Shamsu prefix before streaming starts
        print!("\n  {} ", "◆ Shamsu".bright_cyan().bold());
        let _ = io::stdout().flush();

        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Cannot reach llama.cpp server at {}.\nError: {}", self.base_url, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("LLM server error {}: {}", status, body));
        }

        let mut stream = response.bytes_stream();
        let mut full = String::new();
        let mut tool_calls: Vec<StreamedToolCall> = Vec::new();

        // State machine for detecting ```tool_call ... ``` blocks while streaming
        let mut in_tool_block = false;
        let mut tool_buf = String::new();
        // Buffer for partial fence detection across chunk boundaries
        let mut fence_detector = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { break; }

                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                    for choice in parsed.choices {
                        if let Some(token) = choice.delta.content {
                            full.push_str(&token);
                            fence_detector.push_str(&token);

                            // Check if we just entered a tool_call block
                            if !in_tool_block {
                                if fence_detector.contains("```tool_call") {
                                    in_tool_block = true;
                                    tool_buf.clear();
                                    // Trim everything up to and including the fence marker
                                    let after = fence_detector
                                        .rfind("```tool_call")
                                        .map(|i| fence_detector[i + 12..].to_string())
                                        .unwrap_or_default();
                                    tool_buf.push_str(&after);
                                    fence_detector.clear();
                                    // Don't print tool_call blocks to screen
                                } else {
                                    // Keep only the last 20 chars in detector to catch fences
                                    // split across chunks
                                    if fence_detector.len() > 20 {
                                        let keep = fence_detector.len() - 20;
                                        let visible = &fence_detector[..keep];
                                        // Print the safe portion
                                        print!("{}", visible);
                                        let _ = io::stdout().flush();
                                        fence_detector = fence_detector[keep..].to_string();
                                    }
                                }
                            } else {
                                // Inside a tool_call block — accumulate silently
                                tool_buf.push_str(&token);
                                // Check for closing ```
                                if tool_buf.contains("\n```") || tool_buf.trim_end().ends_with("```") {
                                    // Extract the JSON before the closing fence
                                    let json_part = if let Some(end) = tool_buf.find("\n```") {
                                        tool_buf[..end].trim().to_string()
                                    } else if let Some(end) = tool_buf.rfind("```") {
                                        tool_buf[..end].trim().to_string()
                                    } else {
                                        tool_buf.trim().to_string()
                                    };

                                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_part) {
                                        if let (Some(tool), Some(args)) = (v["tool"].as_str(), v.get("args")) {
                                            tool_calls.push(StreamedToolCall {
                                                tool: tool.to_string(),
                                                args: args.clone(),
                                            });
                                        }
                                    }
                                    in_tool_block = false;
                                    tool_buf.clear();
                                    fence_detector.clear();
                                }
                            }
                        }
                        if choice.finish_reason.is_some() { break; }
                    }
                }
            }
        }

        // Flush any remaining buffered text
        if !fence_detector.is_empty() && !in_tool_block {
            print!("{}", fence_detector);
        }
        println!("\n");
        let _ = io::stdout().flush();

        Ok((full, tool_calls))
    }

    /// Non-streaming (used for internal tasks like summarization)
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: i32,
    ) -> Result<String> {
        #[derive(Deserialize)]
        struct Response { choices: Vec<Choice> }
        #[derive(Deserialize)]
        struct Choice { message: Msg }
        #[derive(Deserialize)]
        struct Msg { content: String }

        let request = ChatCompletionRequest {
            messages, stream: false, temperature, top_p: 0.95, max_tokens,
            stop: vec!["<|endoftext|>".into(), "</s>".into()],
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send().await
            .map_err(|e| anyhow!("Cannot reach server: {}", e))?
            .json::<Response>().await?;

        Ok(resp.choices.into_iter().next().map(|c| c.message.content).unwrap_or_default())
    }
}

impl Default for LlmClient {
    fn default() -> Self { Self::new() }
}
