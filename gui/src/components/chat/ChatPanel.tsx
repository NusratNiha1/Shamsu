import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { useApp } from '../../store/AppContext';
import { useAgent } from '../../hooks/useAgent';
import { MessageRow } from './MessageRow';
import { CookingIndicator } from './CookingIndicator';
import type { Session } from '../../store/appStore';
import { nanoid } from '../../lib/nanoid';

export function ChatPanel() {
  const { state, dispatch } = useApp();
  const { sendMessage, isGenerating } = useAgent();
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const session = state.sessions.find(s => s.id === state.activeSessionId);

  // Auto-scroll on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [session?.messages.length, isGenerating]);

  // Auto-resize textarea
  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = `${Math.min(ta.scrollHeight, 200)}px`;
  }, [input]);

  function handleKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  async function handleSend() {
    const text = input.trim();
    if (!text || isGenerating) return;
    setInput('');
    await sendMessage(text);
  }

  // Create a session automatically if none exists
  function ensureSession() {
    if (!state.activeSessionId) {
      const s: Session = {
        id: nanoid(),
        name: 'New Chat',
        workspace: '',
        messages: [],
        createdAt: Date.now(),
        updatedAt: Date.now(),
      };
      dispatch({ type: 'CREATE_SESSION', session: s });
    }
  }

  if (!session) {
    return (
      <div className="chat-panel">
        <div className="empty-state" style={{ flex: 1 }}>
          <div className="empty-state__icon">◆</div>
          <div className="empty-state__title">Shamsu</div>
          <div className="empty-state__hint">Offline-first AI developer assistant</div>
          <button className="btn btn--primary" onClick={ensureSession}>
            Start a new chat
          </button>
          {!state.serverOnline && (
            <div className="badge badge--err" style={{ marginTop: 8 }}>
              ⚠ LLM server not running — start llama-server.exe first
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="chat-panel">
      {/* Messages */}
      <div className="chat-messages selectable">
        {session.messages.length === 0 && (
          <div className="empty-state">
            <div className="empty-state__icon">◆</div>
            <div className="empty-state__title">{session.name}</div>
            <div className="empty-state__hint">Ask me to build something, explain code, or edit files.</div>
          </div>
        )}

        {session.messages.map(msg => (
          <MessageRow key={msg.id} message={msg} />
        ))}

        {isGenerating && (
          <div className="message-row">
            <div className="message-avatar message-avatar--ai">S</div>
            <CookingIndicator phase={state.cookingPhase} seconds={state.cookingSeconds} />
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="chat-input-area">
        <div className="chat-input-row">
          <textarea
            ref={textareaRef}
            className="chat-input selectable"
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={isGenerating ? 'Generating…' : 'Message Shamsu…'}
            rows={1}
            disabled={isGenerating}
          />
          <button
            className="chat-input-send"
            onClick={handleSend}
            disabled={!input.trim() || isGenerating}
            title="Send (Enter)"
          >
            ↑
          </button>
        </div>
        <div className="chat-input-meta">
          <span>{state.serverOnline ? `◆ ${state.modelName || 'connected'}` : '⚠ server offline'}</span>
          <span style={{ marginLeft: 'auto' }}>Enter to send · Shift+Enter for new line</span>
        </div>
      </div>
    </div>
  );
}
