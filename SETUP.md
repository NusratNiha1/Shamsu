# Shamsu — Setup & Run Guide

> **Shamsu** is an offline-first AI developer assistant that runs entirely on your machine.  
> No internet required after setup. No API keys. No subscriptions.

---

## Table of Contents

1. [System Requirements](#1-system-requirements)
2. [Install Rust](#2-install-rust)
3. [Install the C Linker (Windows)](#3-install-the-c-linker-windows)
4. [Build the CLI](#4-build-the-cli)
5. [Install the Binary](#5-install-the-binary-optional)
6. [Download a Model](#6-download-a-model)
7. [Start the llama.cpp Server](#7-start-the-llamacpp-server)
8. [Configure Shamsu](#8-configure-shamsu)
9. [Run Shamsu](#9-run-shamsu)
10. [Command Reference](#10-command-reference)
11. [Troubleshooting](#11-troubleshooting)

---

## 1. System Requirements

| Component | Minimum | Recommended |
|---|---|---|
| RAM | 8 GB | 16 GB |
| Disk space | 8 GB free | 16 GB free |
| OS | Windows 10/11, Linux, macOS | Windows 11 / Ubuntu 22+ |
| CPU | x86-64 with AVX2 | Modern Intel/AMD |
| GPU | Not required | NVIDIA (CUDA) for faster inference |

---

## 2. Install Rust

Shamsu is written in Rust. You need the Rust toolchain to build it.

### Windows

1. Download and run the installer from **https://rustup.rs**
2. In the installer, choose **option 1** (default installation)
3. After it finishes, **close and reopen your terminal**
4. Verify:

```powershell
rustc --version
cargo --version
```

Expected output (version may differ):
```
rustc 1.96.0 (ac68faa20 2026-05-25)
cargo 1.96.0 (30a34c682 2026-05-25)
```

### Linux / macOS

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version
```

---

## 3. Install the C Linker (Windows only)

Rust on Windows requires a C linker. The easiest option is **MSYS2 with MinGW**.

> **Skip this step on Linux or macOS** — GCC is already available.

### Install MSYS2

```powershell
winget install --id MSYS2.MSYS2 --accept-package-agreements --accept-source-agreements
```

Or download manually from **https://www.msys2.org/**

### Install MinGW GCC inside MSYS2

Open **MSYS2 UCRT64** (from the Start Menu) and run:

```bash
pacman -S --noconfirm mingw-w64-x86_64-gcc
```

### Add MinGW to your PATH

Run this in PowerShell (permanent — survives reboots):

```powershell
$current = [Environment]::GetEnvironmentVariable("PATH", "User")
[Environment]::SetEnvironmentVariable("PATH", "$current;C:\msys64\mingw64\bin", "User")
```

Then **close and reopen** your terminal.

### Switch Rust to the GNU toolchain

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

Verify:
```powershell
rustup show
# Should show: stable-x86_64-pc-windows-gnu (active)
```

---

## 4. Build the CLI

```powershell
cd C:\Users\nusra\Documents\CSE327_Project\cli
cargo build --release
```

The first build downloads all dependencies and compiles everything — this takes **3–5 minutes**.  
Subsequent builds are fast (under 10 seconds for small changes).

When complete, the binary is at:
```
cli\target\release\shamsu.exe
```

---

## 5. Install the Binary (optional)

To run `shamsu` from any directory without specifying the full path:

```powershell
Copy-Item cli\target\release\shamsu.exe "$env:USERPROFILE\.cargo\bin\shamsu.exe" -Force
```

Verify:
```powershell
shamsu --version
# shamsu 0.1.0
```

---

## 6. Download a Model

Shamsu uses **Qwen2.5-Coder-7B-Instruct** in GGUF format. This is a 4.5 GB file.

### Option A — Browser download (simplest)

Click this link and save the file:  
**https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/qwen2.5-coder-7b-instruct-q4_k_m.gguf**

Save it to a memorable location, for example:
```
C:\Users\<your-name>\models\qwen2.5-coder-7b-instruct-q4_k_m.gguf
```

### Option B — PowerShell

```powershell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\models"
Invoke-WebRequest `
  -Uri "https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/qwen2.5-coder-7b-instruct-q4_k_m.gguf" `
  -OutFile "$env:USERPROFILE\models\qwen2.5-coder-7b-instruct-q4_k_m.gguf"
```

### Option C — huggingface-cli (best for slow/interrupted connections)

```powershell
pip install huggingface_hub
huggingface-cli download Qwen/Qwen2.5-Coder-7B-Instruct-GGUF `
  qwen2.5-coder-7b-instruct-q4_k_m.gguf `
  --local-dir "$env:USERPROFILE\models"
```

This supports resuming interrupted downloads automatically.

---

## 7. Start the llama.cpp Server

The llama.cpp binaries are in the `llama.cpp/` folder of this project.

**Open a dedicated terminal** (keep it open while using Shamsu) and run:

```powershell
cd C:\Users\nusra\Documents\CSE327_Project\llama.cpp

.\llama-server.exe `
  -m "$env:USERPROFILE\models\qwen2.5-coder-7b-instruct-q4_k_m.gguf" `
  --port 8080 `
  -c 8192 `
  --gpu-layers 0
```

### Flags explained

| Flag | Meaning |
|---|---|
| `-m <path>` | Path to your `.gguf` model file |
| `--port 8080` | Port for the HTTP API (Shamsu connects here) |
| `-c 8192` | Context window size in tokens |
| `--gpu-layers 0` | CPU-only mode. Safe default for 8 GB RAM |

### If you have an NVIDIA GPU

Replace `--gpu-layers 0` with a higher number to offload layers to the GPU:

```powershell
--gpu-layers 28    # offload most layers — much faster
--gpu-layers 35    # offload all layers for Qwen 7B
```

### Successful startup looks like

```
llama server listening at http://127.0.0.1:8080
```

Leave this terminal open. Shamsu talks to this server for all AI responses.

---

## 8. Configure Shamsu

In a **new terminal**, run:

```powershell
# Verify the server is reachable
shamsu status
```

Expected output:
```
── Shamsu Status ────────────────────────────────
  LLM server: http://127.0.0.1:8080
  Connection: ✓ reachable
  Model:      qwen2.5-coder-7b-instruct
  Database:   C:\Users\<name>\.shamsu\shamsu.db
─────────────────────────────────────────────────
```

### Optional configuration

```powershell
shamsu config set llm_url    http://127.0.0.1:8080   # default, only change if using a different port
shamsu config set temperature 0.7                     # 0.0 = deterministic, 1.0 = creative
shamsu config set max_tokens  2048                    # max response length
shamsu config set stream      true                    # stream tokens as they generate

shamsu config list   # view all current settings
```

---

## 9. Run Shamsu

### Basic chat

```powershell
# Start from your project directory — Shamsu detects the workspace automatically
cd C:\path\to\your\project
shamsu chat
```

### Named session

```powershell
shamsu chat --session my-project
```

### With a permission profile

```powershell
shamsu chat --session my-project --profile standard
# Profiles: safe (read-only) | standard (read+write) | full (read+write+shell)
```

### Single-message mode (non-interactive)

```powershell
shamsu chat -m "what does this codebase do?"
shamsu chat -m "write unit tests for storage.rs"
```

### Inside the chat — slash commands

| Command | What it does |
|---|---|
| `/help` | Show all available commands |
| `/clear` | Clear active context (history is archived, not deleted) |
| `/skills` | List available skills |
| `/profile` | Show the active permission profile |
| `/status` | Check llama.cpp server connectivity |
| `/inspect` | Show workspace detection results |
| `/exit` | Quit Shamsu |

---

## 10. Command Reference

### Sessions

```powershell
shamsu session new "my-app"              # create and activate a session
shamsu session list                      # list all sessions
shamsu session current                   # show active session details
shamsu session switch my-app             # switch to a session
shamsu session rename my-app new-name    # rename a session
shamsu session delete old-session        # delete a session
shamsu session export my-app             # export chat history to my-app.txt
shamsu session set-profile my-app full   # change permission profile
```

### Skills

Skills are reusable instruction sets that shape how Shamsu responds.

```powershell
shamsu skills list                    # list all available skills
shamsu skills show coding             # preview a skill's prompt
shamsu skills activate coding         # activate for current session
shamsu skills activate write-tests    # stack multiple skills
shamsu skills deactivate coding       # remove from current session
shamsu skills new my-skill            # create a custom skill interactively
```

Built-in skills seeded on first run:

| Skill | Purpose |
|---|---|
| `coding` | General expert coding assistant |
| `write-tests` | Test generation specialist |
| `refactor` | Code quality and refactoring |
| `explain` | Explains code in plain language |
| `docs` | Documentation writer |

Custom skills are stored as `.yaml` files in `~/.shamsu/skills/`.

### Configuration

```powershell
shamsu config list                          # show all settings
shamsu config get llm_url                   # get a single value
shamsu config set temperature 0.5           # set a value
shamsu config unset stream                  # remove a setting (revert to default)
```

### Workspace inspection

```powershell
shamsu inspect                               # inspect current directory
shamsu inspect --workspace C:\path\to\proj  # inspect a specific path
```

Shamsu auto-detects one of three workspace modes:
- **Existing project** — has source files or git history → acts as a co-developer
- **Requirements document** — finds `requirements.md`, `PRD.md`, `spec.txt`, etc. → reads it and builds from it
- **Blank workspace** — generates architecture docs first, then implements

---

## 11. Troubleshooting

### `shamsu status` shows "✗ not reachable"

The llama.cpp server is not running. Make sure you started it in a separate terminal (Step 7) and it printed `listening at http://127.0.0.1:8080`.

If using a different port:
```powershell
shamsu config set llm_url http://127.0.0.1:YOUR_PORT
```

### Build fails: `linker 'link.exe' not found` or `error calling dlltool`

The C linker is missing or not on your PATH. Re-do [Step 3](#3-install-the-c-linker-windows) and make sure you:
1. Installed MSYS2 and ran `pacman -S mingw-w64-x86_64-gcc`
2. Added `C:\msys64\mingw64\bin` to your user PATH
3. Switched Rust to `stable-x86_64-pc-windows-gnu`
4. Opened a **fresh** terminal after changing PATH

### Build fails: `cargo` not found

Rust isn't on your PATH. Run:
```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```
Or close and reopen your terminal after installing Rust.

### Model loads but responses are very slow

- Expected speed on CPU-only: 2–8 tokens/second for a 7B Q4 model
- To speed up, use GPU offloading (`--gpu-layers 28` or higher) if you have an NVIDIA GPU
- Reduce context: use `-c 4096` instead of `-c 8192` in the server command

### Out of memory / server crashes

- Reduce `--gpu-layers` (or set to `0` for CPU-only)
- Close other applications to free RAM
- Use a smaller model: `qwen2.5-coder-3b-instruct-q4_k_m.gguf` (~2 GB)

### Session data / history location

Everything is stored in:
```
C:\Users\<your-name>\.shamsu\
├── shamsu.db      ← all sessions, messages, config
├── skills\        ← skill YAML files
└── history.txt    ← readline history
```

To reset everything:
```powershell
Remove-Item -Recurse -Force "$env:USERPROFILE\.shamsu"
```

---

## Quick Start Checklist

- [ ] Rust installed (`rustc --version` works)
- [ ] MSYS2 + MinGW installed (Windows only)
- [ ] Rust switched to GNU toolchain (Windows only)
- [ ] `cargo build --release` completed in `cli/`
- [ ] `shamsu.exe` copied to `~/.cargo/bin/`
- [ ] Model `.gguf` file downloaded
- [ ] `llama-server.exe` running in a separate terminal
- [ ] `shamsu status` shows ✓ reachable
- [ ] `shamsu chat` — you're live

---

*Shamsu — built for developers who own their tools.*
