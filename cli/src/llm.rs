use anyhow::{anyhow, Result};
use colored::Colorize;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default llama.cpp server base URL
const DEFAULT_LLM_URL: &str = "http://127.0.0.1:8080";

/// Returns the configured LLM server URL (from config or default)
pub fn llm_url() -> String {
    crate::storage::get_config("llm_url")
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_LLM_URL.to_string())
}

// ─── Request / Response types ─────────────────────────────────────────────────

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
struct ModelInfo {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

// ─── Client ───────────────────────────────────────────────────────────────────

pub struct LlmClient {
    client: Client,
    base_url: String,
}

impl LlmClient {
    pub fn new() -> Self {
        LlmClient {
            client: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: llm_url(),
        }
    }

    /// Check if the llama.cpp server is reachable
    pub async fn is_alive(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Get the model name from the server
    pub async fn model_name(&self) -> Result<String> {
        let resp = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .send()
            .await?
            .json::<ModelsResponse>()
            .await?;
        Ok(resp
            .data
            .into_iter()
            .next()
            .map(|m| m.id)
            .unwrap_or_else(|| "unknown".to_string()))
    }

    /// Send a chat completion request and stream the response to stdout.
    /// Returns the full assistant response as a String.
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
            stop: vec!["<|endoftext|>".to_string(), "</s>".to_string()],
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                anyhow!(
                    "Cannot reach llama.cpp server at {}. Is it running?\nError: {}",
                    self.base_url,
                    e
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("LLM server error {}: {}", status, body));
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::new();

        print!("{}", "Shamsu: ".bright_green().bold());

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                        for choice in parsed.choices {
                            if let Some(content) = choice.delta.content {
                                print!("{}", content);
                                full_response.push_str(&content);
                            }
                            if choice.finish_reason.is_some() {
                                break;
                            }
                        }
                    }
                }
            }
        }

        println!(); // newline after response
        Ok(full_response)
    }

    /// Non-streaming chat completion (used for summarization, internal tasks)
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: i32,
    ) -> Result<String> {
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: MessageContent,
        }
        #[derive(Deserialize)]
        struct MessageContent {
            content: String,
        }

        let request = ChatCompletionRequest {
            messages,
            stream: false,
            temperature,
            top_p: 0.95,
            max_tokens,
            stop: vec!["<|endoftext|>".to_string(), "</s>".to_string()],
        };

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                anyhow!(
                    "Cannot reach llama.cpp server at {}. Is it running?\nError: {}",
                    self.base_url,
                    e
                )
            })?
            .json::<Response>()
            .await?;

        Ok(resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default())
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}
