use anyhow::{anyhow, Result};
use colored::Colorize;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

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
        // A background OS thread ticks the spinner every 80ms until the
        // AtomicBool is set. We store the JoinHandle in an Option so we
        // can take() it exactly once when stopping.
        let done_flag = Arc::new(AtomicBool::new(false));
        let done_clone = Arc::clone(&done_flag);
        let mut spinner: Option<std::thread::JoinHandle<()>> = Some(std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let start = Instant::now();
            let mut i = 0usize;
            while !done_clone.load(Ordering::Relaxed) {
                let secs = start.elapsed().as_secs();
                print!(
                    "\r  {} {}  {}   ",
                    frames[i % frames.len()].bright_cyan(),
                    "Shamsu is thinking…".bright_cyan().bold(),
                    format!("{}s", secs).truecolor(90, 90, 110),
                );
                let _ = io::stdout().flush();
                std::thread::sleep(Duration::from_millis(80));
                i += 1;
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

        // ── Stream tokens ─────────────────────────────────────────────────
        let mut stream = response.bytes_stream();
        let mut full = String::new();
        let mut first_token = true;

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
                            if first_token {
                                stop_spinner!(spinner);
                                println!("\n  {}", "◆ Shamsu".bright_cyan().bold());
                                print!("  ");
                                first_token = false;
                            }
                            for (i, seg) in content.split('\n').enumerate() {
                                if i > 0 { print!("\n  "); }
                                print!("{}", seg);
                            }
                            let _ = io::stdout().flush();
                            full.push_str(&content);
                        }
                        if choice.finish_reason.is_some() { break; }
                    }
                }
            }
        }

        if first_token {
            stop_spinner!(spinner);
            println!("  {}", "(no response)".dimmed());
        } else {
            println!("\n");
        }

        Ok(full)
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
