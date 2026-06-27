# llama.cpp runtime

This directory holds the llama.cpp release binaries. They are **not tracked by git** (too large).

## Download

Get the latest Windows release from:
https://github.com/ggerganov/llama.cpp/releases

Look for a zip named like `llama-b...-bin-win-avx2-x64.zip` and extract it here.

## Required files

At minimum you need:
- `llama-server.exe` — the inference server shamsu connects to
- `llama.dll`, `ggml.dll`, `ggml-base.dll` — core libraries
- `ggml-cpu-*.dll` — CPU dispatch libraries (include all of them)

## Start the server

```powershell
.\llama-server.exe -m C:\path\to\model.gguf --port 8080 -c 8192 --gpu-layers 0
```
