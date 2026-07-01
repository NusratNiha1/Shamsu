// Thin wrapper around Tauri invoke — falls back to HTTP fetch when running
// in a plain browser (for development without the desktop shell).

import type { ChatMessage } from '../store/appStore';

declare global {
  interface Window {
    __TAURI__?: {
      core: { invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> };
    };
  }
}

function isTauri(): boolean {
  return typeof window !== 'undefined' && !!window.__TAURI__;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke<T>(cmd, args);
  }
  // Browser fallback — proxy through the llama.cpp server directly
  throw new Error(`Tauri not available. Command: ${cmd}`);
}

export function useTauri() {
  async function checkServer(llmUrl: string): Promise<boolean> {
    if (isTauri()) {
      return invoke<boolean>('check_server', { llmUrl });
    }
    try {
      const r = await fetch(`${llmUrl}/health`, { signal: AbortSignal.timeout(4000) });
      return r.ok;
    } catch {
      return false;
    }
  }

  async function getModelName(llmUrl: string): Promise<string> {
    if (isTauri()) {
      return invoke<string>('get_model_name', { llmUrl });
    }
    try {
      const r = await fetch(`${llmUrl}/v1/models`);
      const j = await r.json();
      return j.data?.[0]?.id ?? 'unknown';
    } catch {
      return 'unknown';
    }
  }

  async function sendMessage(
    messages: ChatMessage[],
    llmUrl: string,
    temperature = 0.7,
    maxTokens = 2048
  ): Promise<string> {
    const payload = messages.map(m => ({ role: m.role, content: m.content }));

    if (isTauri()) {
      return invoke<string>('send_message', { messages: payload, llmUrl, temperature, maxTokens });
    }

    // Browser direct call
    const r = await fetch(`${llmUrl}/v1/chat/completions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        messages: payload,
        stream: false,
        temperature,
        top_p: 0.95,
        max_tokens: maxTokens,
        stop: ['<|endoftext|>', '</s>'],
      }),
    });
    const j = await r.json();
    return j.choices?.[0]?.message?.content ?? '';
  }

  async function writeFile(path: string, content: string): Promise<string> {
    if (isTauri()) {
      return invoke<string>('write_file', { path, content });
    }
    console.log('[browser] writeFile:', path);
    return `Wrote ${content.length} chars to '${path}'`;
  }

  async function readFile(path: string): Promise<string> {
    if (isTauri()) {
      return invoke<string>('read_file', { path });
    }
    throw new Error('readFile not available in browser');
  }

  async function listDir(path: string): Promise<string[]> {
    if (isTauri()) {
      return invoke<string[]>('list_dir', { path });
    }
    return [];
  }

  return { checkServer, getModelName, sendMessage, writeFile, readFile, listDir };
}
