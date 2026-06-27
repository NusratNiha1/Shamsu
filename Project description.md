# Shamsu — Project Description
**An Offline-First, AI-Powered Developer Assistant**
*Version 0.1 — Foundational Spec*

---

## 1. Vision

Shamsu is a fully offline, privacy-first AI coding and productivity assistant that runs locally on consumer hardware (8GB RAM laptops, mobile devices). It is designed to be a complete, self-contained alternative to cloud-dependent tools like Claude Code — with no API calls, no subscriptions, and no internet requirement after installation.

Shamsu understands *context* — whether you open it in an existing codebase, drop in a requirements document, or start from a blank prompt, it knows what mode to operate in and behaves accordingly.

---

## 2. Core Philosophy

| Principle | Description |
|---|---|
| **Offline-first** | All inference runs locally. No data leaves the device. |
| **Context-aware** | Detects workspace state automatically (existing project, docs, blank slate) |
| **Self-contained** | Installation includes all dependencies; nothing requires manual setup |
| **Incremental delivery** | CLI first → GUI later; each phase ships usable value |
| **Cross-platform** | Mac, Windows, Linux, Android, iOS from a single codebase |

---

## 3. Recommended Local Model

For a 4B parameter model that runs well on 8GB RAM:

| Model | Size (Q4) | Context | Strengths |
|---|---|---|---|
| **Qwen2.5-Coder-7B-Instruct (Q4)** | ~4.5GB | 128K tokens | Best-in-class for coding at small scale |
| **Phi-4-mini (3.8B)** | ~2.5GB | 16K tokens | Fast, excellent reasoning |
| **Llama-3.2-3B-Instruct (Q4)** | ~2GB | 8K tokens | Reliable, broad task coverage |
| **DeepSeek-Coder-V2-Lite (Q4)** | ~5GB | 128K tokens | Strong code generation |

**Recommendation: Qwen2.5-Coder-7B-Instruct (Q4_K_M)** — best coding performance in the 4–7B range, fits in 8GB RAM with room for the OS, and supports a 128K token context natively.

**Runtime: [llama.cpp](https://github.com/ggerganov/llama.cpp)** via its server mode, called locally from Node/Rust.

---

## 4. Technology Stack

| Layer | Technology | Reason |
|---|---|---|
| **CLI core** | Rust | Performance, cross-platform binary distribution, memory safety |
| **API/orchestration layer** | Node.js | MCP protocol support, JS ecosystem, rapid tooling |
| **GUI (Phase 2)** | React + Tauri | Web UI with Rust shell; single codebase for desktop + mobile |
| **LLM runtime** | llama.cpp | Offline inference, GGUF model support |
| **Storage** | SQLite (via `rusqlite`) | Sessions, memory, permissions — embedded, no server needed |
| **Voice I/O** | Whisper.cpp (STT) + Coqui/Piper (TTS) | Local, no cloud |

---

## 5. Feature Set

### 5.1 Context Detection (Workspace Intelligence)

Shamsu auto-detects the workspace state when launched in a directory:

| Scenario | What Shamsu Does |
|---|---|
| **Existing codebase** (has source files, git history) | Reads structure, infers stack, operates as a co-developer on the existing project |
| **Requirements doc present** (`requirements.md`, `PRD.md`, `spec.txt`, etc.) | Reads the doc, generates a project plan, then starts building from scratch |
| **Blank workspace + text prompt** | Generates documentation first (architecture, data models, file structure), gets confirmation if needed, then implements |

This is the core differentiator — the user never has to tell Shamsu "this is an existing project" or "start from scratch."

### 5.2 Infinite (Disk-Backed) Context Window

- Context is not limited by model token window
- A sliding/summarization engine manages what fits in the active context
- Full conversation and file history is stored on disk (SQLite + flat files)
- Older context is summarized and re-injected as compressed memory
- Effective context length = available disk space

### 5.3 Multi-Session Support

- Named, persistent sessions (e.g., `shamsu session new "todo-app"`)
- Switch between sessions without losing state
- Each session stores: conversation history, workspace path, active files, memory snapshots
- Session list, rename, delete, export as supported commands

### 5.4 MCP (Model Context Protocol) Support

- Compatible with the MCP standard for tool use
- Users can register MCP servers (local or networked)
- Built-in MCP tools: file read/write, shell execution, web fetch (local proxy), search
- Third-party MCP servers plug in via config

### 5.5 Skills System

- Skills are reusable, named instruction sets (similar to system prompt modules)
- Stored as `.yaml` or `.md` files in `~/.shamsu/skills/`
- Examples: `coding-react`, `write-tests`, `refactor-clean`, `explain-simple`
- Skills can be stacked and activated per session or per command

### 5.6 Permission Management

- File system permissions: read-only vs read-write, scoped to directories
- Shell execution: off by default, user must explicitly enable per session
- Network access: disabled by default
- Permission profiles: `safe`, `standard`, `full` — user selects on session start
- All destructive actions (file writes, shell commands) are logged and reversible via dry-run mode

### 5.7 Voice Interaction

- **Speech-to-text**: Whisper.cpp (runs locally, no cloud)
- **Text-to-speech**: Piper TTS (fast, natural, offline)
- CLI voice mode: `shamsu --voice`
- Push-to-talk or always-on modes
- Voice commands are transcribed, processed as normal input, response read aloud

### 5.8 Coding Capabilities

- Understands and works in: Python, JavaScript/TypeScript, Rust, Go, Java, C/C++, and more
- File-level and project-level edits
- Reads imports, dependencies (`package.json`, `Cargo.toml`, `pyproject.toml`, etc.)
- Can run code and capture output (with permission)
- Diff-based edits (shows changes before applying)
- Test generation and execution

### 5.9 Documentation-First Workflow

When given a prompt on a blank workspace:
1. **Plan** — Generate `ARCHITECTURE.md`, `DATA_MODEL.md`, `FILE_STRUCTURE.md`
2. **Confirm** — Show the plan, ask for approval (or auto-proceed with `--yes` flag)
3. **Implement** — Build the project file by file, with progress output
4. **Verify** — Run tests if possible, summarize what was built

---

## 6. Context Engineering Strategy

Shamsu uses a layered context system:

```
┌─────────────────────────────────────────┐
│  SYSTEM LAYER (always present)          │  ← Shamsu identity, active skill, permissions
├─────────────────────────────────────────┤
│  WORKSPACE LAYER (per session)          │  ← Project structure, key files, git info
├─────────────────────────────────────────┤
│  MEMORY LAYER (compressed history)      │  ← Summarized past conversation
├─────────────────────────────────────────┤
│  ACTIVE LAYER (recent turns + files)    │  ← Last N messages + currently open files
└─────────────────────────────────────────┘
```

- Files are chunked and embedded for retrieval (local vector store using `sqlite-vec` or `hnswlib`)
- Relevant chunks are pulled into context dynamically based on the current query
- Memory is periodically compressed and archived to disk

---

## 7. Installation & Distribution

- Single installer per platform (`.dmg`, `.exe`, `.AppImage`, `.deb`, `.apk`, `.ipa`)
- Installer bootstraps: llama.cpp runtime, Whisper.cpp, Piper TTS, SQLite, Node runtime (bundled)
- Model is downloaded on first launch (user selects from a curated list, or brings their own GGUF)
- No system dependencies required from the user
- Auto-update system for Shamsu itself (model weights are separate)

---

## 8. Platform Support

| Platform | Phase 1 (CLI) | Phase 2 (GUI) |
|---|---|---|
| Linux | ✅ | ✅ |
| macOS | ✅ | ✅ |
| Windows | ✅ | ✅ |
| Android | ⚠️ (shell/Termux) | ✅ (Tauri mobile) |
| iOS | ❌ | ✅ (Tauri mobile) |

---

## 9. Development Phases

### Phase 1 — CLI (Current Focus)
- [ ] Rust CLI skeleton with `clap` argument parsing
- [ ] llama.cpp integration (local HTTP server mode)
- [ ] Basic chat loop with session persistence (SQLite)
- [ ] File read/write tools with permission gates
- [ ] Context window manager (sliding window + summarization)
- [ ] Workspace detection logic
- [ ] MCP client implementation
- [ ] Skills loader
- [ ] Voice I/O (Whisper + Piper)
- [ ] Cross-platform binary builds (GitHub Actions)

### Phase 2 — GUI
- [ ] React + Tauri desktop app
- [ ] Session browser, file tree, diff viewer
- [ ] Visual permission management
- [ ] Model switcher and settings UI
- [ ] Mobile builds (Android + iOS via Tauri)

### Phase 3 — Ecosystem
- [ ] Shamsu plugin/skill marketplace (local, no cloud)
- [ ] Team sync via local network (LAN-only, no cloud)
- [ ] Fine-tuning workflow for personal model adaptation

---

## 10. Project Structure (Phase 1)

```
shamsu/
├── cli/                        # Rust CLI (core binary)
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/           # clap subcommands
│   │   ├── context/            # Context engine, memory, summarization
│   │   ├── workspace/          # Project detection, file indexing
│   │   ├── llm/                # llama.cpp HTTP client
│   │   ├── mcp/                # MCP client/server bridge
│   │   ├── skills/             # Skill loader and manager
│   │   ├── permissions/        # Permission system
│   │   ├── voice/              # Whisper + Piper integration
│   │   └── storage/            # SQLite session store
│   └── Cargo.toml
├── orchestrator/               # Node.js layer (MCP host, tool routing)
│   ├── src/
│   │   ├── index.ts
│   │   ├── mcp-host.ts
│   │   └── tools/
│   └── package.json
├── gui/                        # React + Tauri (Phase 2)
├── models/                     # Model download scripts, GGUF configs
├── skills/                     # Built-in skill definitions
├── installer/                  # Platform-specific installer scripts
└── README.md
```

---

## 11. Key Differentiators vs Claude Code / Cursor / Copilot

| Feature | Shamsu | Claude Code | Cursor | GitHub Copilot |
|---|---|---|---|---|
| Fully offline | ✅ | ❌ | ❌ | ❌ |
| No subscription | ✅ | ❌ | ❌ | ❌ |
| Local model | ✅ | ❌ | Partial | ❌ |
| Disk-backed context | ✅ | ❌ | ❌ | ❌ |
| Voice I/O | ✅ | ❌ | ❌ | ❌ |
| MCP support | ✅ | ✅ | ❌ | ❌ |
| Mobile (Android/iOS) | ✅ | ❌ | ❌ | ❌ |
| Doc-first workflow | ✅ | ❌ | ❌ | ❌ |
| Self-installing | ✅ | Partial | ✅ | ✅ |

---

*Shamsu — Built for developers who own their tools.*