use anyhow::Result;
use colored::Colorize;
use crate::llm::LlmClient;

/// Check whether the llama.cpp server is reachable and print a status report
pub async fn check() -> Result<()> {
    let llm = LlmClient::new();
    let url = crate::llm::llm_url();

    println!("\n{}", "── Shamsu Status ────────────────────────────────".dimmed());
    println!("  {} {}", "LLM server:".bold(), url.dimmed());

    print!("  {} ", "Connection:".bold());
    if llm.is_alive().await {
        println!("{}", "✓ reachable".bright_green().bold());

        // Try to get model name
        match llm.model_name().await {
            Ok(name) => {
                println!("  {} {}", "Model:".bold(), name.bright_cyan());
            }
            Err(e) => {
                println!("  {} {}", "Model:".bold(), format!("(could not query: {})", e).dimmed());
            }
        }
    } else {
        println!("{}", "✗ not reachable".bright_red().bold());
        println!(
            "\n  {}",
            "Start the llama.cpp server first. See the guide below:".yellow()
        );
        print_server_guide(&url);
    }

    // Storage status
    let db = crate::storage::db_path();
    println!(
        "  {} {}",
        "Database:".bold(),
        if db.exists() {
            db.to_string_lossy().to_string().bright_green().to_string()
        } else {
            "not initialized".dimmed().to_string()
        }
    );

    println!("{}", "─────────────────────────────────────────────────".dimmed());
    Ok(())
}

fn print_server_guide(url: &str) {
    let default_port = url
        .split(':')
        .last()
        .unwrap_or("8080");

    println!(
        "\n  {}",
        "─── How to start llama.cpp server ───".bright_cyan()
    );
    println!("  1. Download llama.cpp: https://github.com/ggerganov/llama.cpp/releases");
    println!("  2. Download a GGUF model (e.g. Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf)");
    println!("     from: https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF");
    println!("  3. Run the server:");
    println!(
        "     {}",
        format!(
            "llama-server -m path/to/model.gguf --port {} -c 8192 --gpu-layers 0",
            default_port
        )
        .bright_white()
    );
    println!("  4. Then run: shamsu chat");
    println!("\n  Or configure a different server URL:");
    println!("     {}", "shamsu config set llm_url http://127.0.0.1:8080".bright_white());
}
