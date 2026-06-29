use serde::{Deserialize, Serialize};
use tauri::Manager;

// ── Types mirroring the Shamsu CLI data model ─────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub workspace: String,
    pub is_active: bool,
    pub permission_profile: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageArgs {
    pub session_id: String,
    pub content: String,
    pub llm_url: String,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Check whether the llama.cpp server is reachable.
#[tauri::command]
pub async fn check_server(llm_url: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;
    let ok = client
        .get(format!("{}/health", llm_url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    Ok(ok)
}

/// Get the currently loaded model name.
#[tauri::command]
pub async fn get_model_name(llm_url: String) -> Result<String, String> {
    #[derive(Deserialize)]
    struct ModelsResp { data: Vec<ModelInfo> }
    #[derive(Deserialize)]
    struct ModelInfo { id: String }

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/v1/models", llm_url))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<ModelsResp>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.data.into_iter().next().map(|m| m.id).unwrap_or_else(|| "unknown".into()))
}

/// Send a chat message (non-streaming). Returns full assistant response.
#[tauri::command]
pub async fn send_message(
    messages: Vec<ChatMessage>,
    llm_url: String,
    temperature: f32,
    max_tokens: i32,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Req {
        messages: Vec<ChatMessage>,
        stream: bool,
        temperature: f32,
        top_p: f32,
        max_tokens: i32,
        stop: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Resp { choices: Vec<Choice> }
    #[derive(Deserialize)]
    struct Choice { message: Msg }
    #[derive(Deserialize)]
    struct Msg { content: String }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let req = Req {
        messages,
        stream: false,
        temperature,
        top_p: 0.95,
        max_tokens,
        stop: vec!["<|endoftext|>".into(), "</s>".into()],
    };

    let resp = client
        .post(format!("{}/v1/chat/completions", llm_url))
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Cannot reach server: {}", e))?
        .json::<Resp>()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(resp
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default())
}

/// Write a file to disk (used by the agent file-writing pipeline).
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<String, String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    let lines = content.lines().count();
    Ok(format!("Wrote {} lines to '{}'", lines, path))
}

/// Read a file from disk.
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Cannot read '{}': {}", path, e))
}

/// List directory contents.
#[tauri::command]
pub async fn list_dir(path: String) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&path)
        .map_err(|e| format!("Cannot list '{}': {}", path, e))?
        .flatten()
    {
        let meta = entry.metadata().ok();
        let is_dir = meta.map(|m| m.is_dir()).unwrap_or(false);
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(if is_dir { format!("{}/", name) } else { name });
    }
    entries.sort();
    Ok(entries)
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            check_server,
            get_model_name,
            send_message,
            write_file,
            read_file,
            list_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
