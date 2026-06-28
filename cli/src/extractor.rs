/// extractor.rs — Automatic code-block and shell-command extractor.
///
/// After the LLM responds in plain markdown, this module:
///   1. Finds every fenced code block in the response
///   2. Determines the target filename from the nearest heading/annotation
///   3. Returns (filename, content) pairs ready to be written to disk
///
/// Shell blocks (```bash / ```sh / ```powershell / ```cmd) are returned
/// separately as commands to execute.
///
/// This runs UNCONDITIONALLY — no tool_call format required from the model.

/// A file write action extracted from the response.
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    pub path: String,
    pub content: String,
}

/// A shell command extracted from a bash/sh/powershell/cmd block.
#[derive(Debug, Clone)]
pub struct ExtractedShell {
    pub command: String,
}

/// Result of scanning one LLM response.
#[derive(Debug, Default)]
pub struct Extracted {
    pub files: Vec<ExtractedFile>,
    pub shell: Vec<ExtractedShell>,
}

/// Map a language tag to a default filename used as last-resort fallback.
fn lang_default_filename(lang: &str) -> Option<&'static str> {
    match lang.to_lowercase().trim() {
        "html"                    => Some("index.html"),
        "css"                     => Some("styles.css"),
        "javascript" | "js"       => Some("script.js"),
        "typescript" | "ts"       => Some("index.ts"),
        "jsx"                     => Some("index.jsx"),
        "tsx"                     => Some("index.tsx"),
        "python" | "py"           => Some("main.py"),
        "rust" | "rs"             => Some("main.rs"),
        "go"                      => Some("main.go"),
        "java"                    => Some("Main.java"),
        "c"                       => Some("main.c"),
        "cpp" | "c++"             => Some("main.cpp"),
        "json"                    => Some("config.json"),
        "toml"                    => Some("Cargo.toml"),
        "yaml" | "yml"            => Some("config.yaml"),
        "sh" | "bash" | "shell"   => Some("run.sh"),
        "sql"                     => Some("schema.sql"),
        "xml"                     => Some("config.xml"),
        "md" | "markdown"         => Some("README.md"),
        _                         => None,
    }
}

/// Known extensions list for filename detection.
const KNOWN_EXTS: &[&str] = &[
    "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "h",
    "hpp", "cs", "rb", "php", "swift", "kt", "md", "toml", "yaml", "yml",
    "json", "html", "css", "scss", "sass", "sh", "bash", "zsh", "fish",
    "sql", "txt", "env", "lock", "conf", "cfg", "ini", "xml", "vue", "svelte",
];

fn looks_like_filename(s: &str) -> bool {
    if s.is_empty() || s.len() > 120 { return false; }
    // Must have a dot-separated extension
    if let Some(dot) = s.rfind('.') {
        let ext = &s[dot + 1..];
        if KNOWN_EXTS.contains(&ext) {
            // No spaces allowed in a bare filename (paths can have / or \)
            return !s.contains(' ');
        }
    }
    false
}

/// Try to extract a filename from a single line of text.
/// Handles:
///   - bare filename: `styles.css`
///   - comment prefix: `// src/main.rs`  `# app.py`
///   - annotation: `File: index.html`  `file: index.html`
///   - backtick-wrapped: ``Create `styles.css` ``
///   - bold: `**index.html**`
///   - path: `src/components/App.tsx`
fn extract_filename_from_line(line: &str) -> Option<String> {
    let line = line.trim();

    // Skip markdown headings — they contain prose, not filenames.
    // But DO check backticks inside them (e.g. "#### Create `index.html`")
    // We'll let the backtick scan below handle headings.

    // 1. Backtick-wrapped tokens anywhere on the line: `foo.js`
    {
        let mut search = line;
        while let Some(start) = search.find('`') {
            let rest = &search[start + 1..];
            if let Some(end) = rest.find('`') {
                let token = rest[..end].trim();
                if looks_like_filename(token) {
                    return Some(token.to_string());
                }
                search = &rest[end + 1..];
            } else {
                break;
            }
        }
    }

    // 2. "named X" / "file named X" pattern
    {
        let lower = line.to_lowercase();
        if let Some(pos) = lower.find("named ") {
            let after = line[pos + 6..].trim();
            let token = after
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('`')
                .trim_matches('\'')
                .trim_matches('"')
                .trim_end_matches(':')
                .trim_end_matches(',')
                .trim_end_matches('.');
            if looks_like_filename(token) {
                return Some(token.to_string());
            }
        }
    }

    // 3. Bold: **filename**
    {
        let s = line
            .trim_start_matches("**")
            .trim_end_matches("**")
            .trim();
        if looks_like_filename(s) {
            return Some(s.to_string());
        }
    }

    // 4. Comment / annotation prefixes
    {
        let stripped = line
            .trim_start_matches("//")
            .trim_start_matches('#')
            .trim_start_matches("File:")
            .trim_start_matches("file:")
            .trim_start_matches("→")
            .trim_end_matches(':')
            .trim();
        if looks_like_filename(stripped) {
            return Some(stripped.to_string());
        }
    }

    // 5. Bare path or filename on its own line
    {
        let s = line.trim_end_matches(':').trim();
        if looks_like_filename(s) {
            return Some(s.to_string());
        }
    }

    None
}

/// Scan the lines preceding a fence (up to 8 lines back) for a filename hint.
fn find_filename_before_fence(lines: &[&str], fence_idx: usize) -> Option<String> {
    let start = if fence_idx >= 8 { fence_idx - 8 } else { 0 };
    // Search closest lines first (most likely to be the right filename)
    for idx in (start..fence_idx).rev() {
        if let Some(name) = extract_filename_from_line(lines[idx]) {
            return Some(name);
        }
    }
    None
}

/// Main entry point: parse an LLM response and extract all files + shell commands.
pub fn extract(response: &str) -> Extracted {
    let mut result = Extracted::default();
    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Only process opening fences
        if !trimmed.starts_with("```") {
            i += 1;
            continue;
        }

        // Get the language tag (everything after the opening ```)
        let lang = trimmed[3..].trim().to_lowercase();

        // Skip tool_call blocks — those are handled separately
        if lang == "tool_call" {
            i += 1;
            continue;
        }

        // Find the closing ```
        let content_start = i + 1;
        let mut close = content_start;
        while close < lines.len() && lines[close].trim() != "```" {
            close += 1;
        }

        let code_lines = &lines[content_start..close];
        let content = code_lines.join("\n");

        // Skip empty blocks
        if content.trim().is_empty() {
            i = close + 1;
            continue;
        }

        // ── Shell block? ──────────────────────────────────────────────────
        if matches!(lang.as_str(), "bash" | "sh" | "shell" | "powershell" | "ps1" | "cmd" | "bat") {
            // Each non-empty, non-comment line is a command
            for line in code_lines {
                let cmd = line.trim();
                if !cmd.is_empty() && !cmd.starts_with('#') && !cmd.starts_with("::") {
                    result.shell.push(ExtractedShell { command: cmd.to_string() });
                }
            }
            i = close + 1;
            continue;
        }

        // ── File block ────────────────────────────────────────────────────
        // Try to find a filename from context, then fall back to language default
        let filename = find_filename_before_fence(&lines, i)
            .or_else(|| lang_default_filename(&lang).map(|s| s.to_string()));

        if let Some(path) = filename {
            result.files.push(ExtractedFile {
                path,
                content: content + "\n",
            });
        }

        i = close + 1;
    }

    result
}
