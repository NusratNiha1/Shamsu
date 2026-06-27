// ui.rs — Shamsu terminal UI primitives
//
// Centralises all rendering: boxes, banners, tool cards, diff blocks,
// permission prompts, and the status bar.
#![allow(dead_code)]

use colored::Colorize;
use std::io::{self, Write};

// ─── Palette ──────────────────────────────────────────────────────────────────

pub const W: usize = 72; // content width (box inner)

// ─── Box drawing helpers ──────────────────────────────────────────────────────

pub fn top_bar(label: &str, color_fn: fn(&str) -> colored::ColoredString) -> String {
    let inner = W - 2;
    let pad = inner.saturating_sub(label.len());
    let left = pad / 2;
    let right = pad - left;
    format!(
        "{}{}{}{}{}",
        color_fn("╭"),
        color_fn(&"─".repeat(left)),
        color_fn(label),
        color_fn(&"─".repeat(right)),
        color_fn("╮"),
    )
}

pub fn mid_bar(color_fn: fn(&str) -> colored::ColoredString) -> String {
    format!(
        "{}{}{}",
        color_fn("├"),
        color_fn(&"─".repeat(W - 2)),
        color_fn("┤"),
    )
}

pub fn bot_bar(color_fn: fn(&str) -> colored::ColoredString) -> String {
    format!(
        "{}{}{}",
        color_fn("╰"),
        color_fn(&"─".repeat(W - 2)),
        color_fn("╯"),
    )
}

pub fn side(color_fn: fn(&str) -> colored::ColoredString, content: &str) -> String {
    let inner = W - 2;
    let visible_len = strip_ansi_len(content);
    let pad = inner.saturating_sub(visible_len);
    format!("{} {}{} {}", color_fn("│"), content, " ".repeat(pad), color_fn("│"))
}

/// Rough visible-character length (strips ANSI escape codes for padding calc)
fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

// ─── Banner ───────────────────────────────────────────────────────────────────

pub fn print_banner() {
    println!();
    let lines = [
        "  ░██████╗██╗  ██╗ █████╗ ███╗   ███╗░██████╗██╗   ██╗",
        "  ██╔════╝██║  ██║██╔══██╗████╗ ████║██╔════╝██║   ██║",
        "  ╚█████╗ ███████║███████║██╔████╔██║╚█████╗ ██║   ██║",
        "   ╚═══██╗██╔══██║██╔══██║██║╚██╔╝██║ ╚═══██╗██║   ██║",
        "  ██████╔╝██║  ██║██║  ██║██║ ╚═╝ ██║██████╔╝╚██████╔╝",
        "  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═════╝  ╚═════╝ ",
    ];
    for (i, line) in lines.iter().enumerate() {
        if i < 2 {
            println!("{}", line.bright_cyan().bold());
        } else if i < 4 {
            println!("{}", line.cyan());
        } else {
            println!("{}", line.bright_blue());
        }
    }
    println!(
        "  {}  {}",
        "Offline-first AI Developer Assistant".bright_white().bold(),
        "v0.1.0".dimmed()
    );
    println!("  {}", "Built for developers who own their tools.".truecolor(120, 120, 140));
    println!();
}

// ─── Session header ───────────────────────────────────────────────────────────

pub fn print_session_header(
    session_name: &str,
    workspace_kind: &str,
    profile: &str,
    model: &str,
    skills: &[String],
) {
    let profile_colored = match profile {
        "safe"     => profile.bright_green().bold().to_string(),
        "full"     => profile.bright_red().bold().to_string(),
        _          => profile.bright_yellow().bold().to_string(),
    };

    println!("{}", "╭──────────────────────────────────────────────────────────────────────────╮".truecolor(60, 60, 80));
    println!("{}  {}  {}  {}{}",
        "│".truecolor(60, 60, 80),
        "◆ SHAMSU".bright_cyan().bold(),
        format!("session:{}", session_name).truecolor(180, 180, 200),
        format!("profile:{}", profile_colored),
        "                                                  │".truecolor(60, 60, 80),
    );
    println!("{}  {}  {}{}",
        "│".truecolor(60, 60, 80),
        format!("⬡ {}", workspace_kind).truecolor(160, 200, 160),
        format!("model:{}", model).truecolor(140, 140, 180),
        "                                                              │".truecolor(60, 60, 80),
    );
    if !skills.is_empty() {
        println!("{}  {}{}",
            "│".truecolor(60, 60, 80),
            format!("◈ skills: {}", skills.join(", ")).bright_magenta(),
            "                                                                 │".truecolor(60, 60, 80),
        );
    }
    println!("{}", "╰──────────────────────────────────────────────────────────────────────────╯".truecolor(60, 60, 80));
    println!(
        "  {}",
        "Type a message, or /help for commands. Ctrl+C to interrupt.".truecolor(100, 100, 120)
    );
    println!();
}

// ─── Tool event card ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Running,
    Ok,
    Err,
    Skipped,
}

pub fn print_tool_card(tool: &str, detail: &str, status: ToolStatus) {
    let (icon, label_color): (&str, fn(&str) -> colored::ColoredString) = match status {
        ToolStatus::Running => ("⟳", |s| s.bright_blue().bold()),
        ToolStatus::Ok      => ("✓", |s| s.bright_green().bold()),
        ToolStatus::Err     => ("✗", |s| s.bright_red().bold()),
        ToolStatus::Skipped => ("⊘", |s| s.dimmed()),
    };

    let tool_icon = match tool {
        "read_file"    => "📄",
        "write_file"   => "✏ ",
        "patch_file"   => "⬡ ",
        "delete_file"  => "🗑 ",
        "create_dir"   => "📁",
        "list_dir"     => "📂",
        "run_shell"    => "⚡",
        "search_files" => "🔍",
        _              => "⚙ ",
    };

    let status_str = match status {
        ToolStatus::Running => label_color("running"),
        ToolStatus::Ok      => label_color("done"),
        ToolStatus::Err     => label_color("failed"),
        ToolStatus::Skipped => label_color("skipped"),
    };

    // Truncate detail for display
    let detail_display = if detail.len() > 45 {
        format!("{}…", &detail[..44])
    } else {
        detail.to_string()
    };

    println!(
        "  {} {} {}  {}  {}",
        icon.truecolor(100, 200, 100),
        tool_icon,
        tool.bright_cyan().bold(),
        detail_display.truecolor(180, 180, 200),
        status_str,
    );
}

// ─── Permission prompt ────────────────────────────────────────────────────────

/// Shows a rich permission prompt and reads y/n. Returns true if approved.
pub fn prompt_permission(action: &str, target: &str, detail: Option<&str>) -> bool {
    let (icon, color): (&str, fn(&str) -> colored::ColoredString) = match action {
        "write_file"  | "patch_file" => ("✏ ", |s| s.bright_yellow().bold()),
        "delete_file"                => ("🗑 ", |s| s.bright_red().bold()),
        "run_shell"                  => ("⚡", |s| s.bright_red().bold()),
        "create_dir"                 => ("📁", |s| s.bright_yellow().bold()),
        _                            => ("⚙ ", |s| s.yellow().bold()),
    };

    println!();
    println!("  {}", "╭─ Permission Required ──────────────────────────────────────────────────╮".bright_yellow());
    println!("  {}  {} {}  {}", "│".bright_yellow(), icon, color(action), "│".bright_yellow());
    println!("  {}  {}  {}", "│".bright_yellow(), format!("→ {}", target).bright_white().bold(), "│".bright_yellow());
    if let Some(d) = detail {
        // Print multi-line detail with truncation per line
        for line in d.lines().take(4) {
            let truncated = if line.len() > 65 { format!("{}…", &line[..64]) } else { line.to_string() };
            println!("  {}    {}  {}", "│".bright_yellow(), truncated.truecolor(160, 160, 180), "│".bright_yellow());
        }
    }
    println!("  {}", "╰────────────────────────────────────────────────────────────────────────╯".bright_yellow());
    print!("  {} [{}{}] ", "Allow?".bold(), "y".bright_green().bold(), "/n".dimmed());
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes" | "")
}

// ─── Diff / file write display ────────────────────────────────────────────────

/// Print a compact "file written" confirmation with line count
pub fn print_file_written(path: &str, bytes: usize, lines: usize, is_new: bool) {
    let action = if is_new { "Created" } else { "Updated" };
    let icon   = if is_new { "+" } else { "~" };
    println!(
        "  {} {} {}  {} bytes · {} lines",
        icon.bright_green().bold(),
        action.bright_green(),
        path.bright_white().bold(),
        bytes,
        lines,
    );
}

/// Print deleted file confirmation
pub fn print_file_deleted(path: &str) {
    println!(
        "  {} {} {}",
        "−".bright_red().bold(),
        "Deleted".bright_red(),
        path.bright_white().bold(),
    );
}

/// Print a shell command block before running it
pub fn print_shell_block(command: &str, cwd: Option<&str>) {
    let dir = cwd.unwrap_or(".");
    println!();
    println!("  {}", "┌─ Shell ────────────────────────────────────────────────────────────────┐".truecolor(200, 100, 50));
    println!("  {}  {}  {}", "│".truecolor(200, 100, 50), format!("$ {}", command).bright_white().bold(), "│".truecolor(200, 100, 50));
    println!("  {}  {}  {}", "│".truecolor(200, 100, 50), format!("cwd: {}", dir).truecolor(140, 140, 160), "│".truecolor(200, 100, 50));
    println!("  {}", "└────────────────────────────────────────────────────────────────────────┘".truecolor(200, 100, 50));
}

/// Print shell output
pub fn print_shell_output(output: &str, success: bool) {
    let border_color: fn(&str) -> colored::ColoredString = if success {
        |s| s.truecolor(80, 180, 80)
    } else {
        |s| s.truecolor(220, 80, 80)
    };
    println!("  {}", border_color("  ╷"));
    for line in output.lines().take(20) {
        println!("  {}  {}", border_color("  │"), line.truecolor(200, 200, 210));
    }
    let total_lines = output.lines().count();
    if total_lines > 20 {
        println!("  {}  {}", border_color("  │"), format!("… {} more lines", total_lines - 20).dimmed());
    }
    println!("  {}", border_color("  ╵"));
}

// ─── Thinking indicator (inline, no spinner lib needed) ──────────────────────

pub fn print_thinking() {
    print!("\n  {} ", "◆ Shamsu".bright_cyan().bold());
    let _ = io::stdout().flush();
}

pub fn print_response_prefix() {
    // Called right before streaming starts — newline after thinking dot
    println!();
    print!("  ");
    let _ = io::stdout().flush();
}

// ─── Separator ────────────────────────────────────────────────────────────────

pub fn separator() {
    println!("  {}", "─".repeat(W).truecolor(50, 50, 65));
}

// ─── Error / warning boxes ────────────────────────────────────────────────────

pub fn print_error(msg: &str) {
    println!();
    println!("  {} {}", "✗".bright_red().bold(), msg.bright_red());
}

pub fn print_warning(msg: &str) {
    println!("  {} {}", "⚠".bright_yellow(), msg.yellow());
}

pub fn print_success(msg: &str) {
    println!("  {} {}", "✓".bright_green().bold(), msg.bright_green());
}

pub fn print_info(msg: &str) {
    println!("  {} {}", "ℹ".bright_blue(), msg.truecolor(180, 180, 210));
}

// ─── Help panel ───────────────────────────────────────────────────────────────

pub fn print_help() {
    println!();
    println!("  {}", "╭─ Commands ─────────────────────────────────────────────────────────────╮".truecolor(60, 60, 100));
    let cmds: &[(&str, &str, &str)] = &[
        ("/help",        "h",  "Show this panel"),
        ("/clear",       "cl", "Archive context (history preserved on disk)"),
        ("/undo",        "u",  "Show last tool action taken"),
        ("/skills",      "sk", "List available skills"),
        ("/profile",     "pr", "Show active permission profile"),
        ("/status",      "st", "Check llama.cpp server"),
        ("/inspect",     "in", "Show workspace detection info"),
        ("/compact",     "co", "Force context compression now"),
        ("/exit",        "q",  "Quit"),
    ];
    for (cmd, short, desc) in cmds {
        println!("  {}  {:<14} {}  {}  {}",
            "│".truecolor(60, 60, 100),
            cmd.bright_cyan().bold(),
            format!("({short})").truecolor(80, 80, 120),
            desc.truecolor(180, 180, 200),
            "│".truecolor(60, 60, 100),
        );
    }
    println!("  {}", "╰────────────────────────────────────────────────────────────────────────╯".truecolor(60, 60, 100));
    println!();
}
