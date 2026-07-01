import { useApp } from '../../store/AppContext';
import type { Theme } from '../../store/appStore';
import { useTauri } from '../../hooks/useTauri';
import { useState } from 'react';

const THEMES: { id: Theme; label: string; previewClass: string }[] = [
  { id: 'dark',   label: 'Dark',   previewClass: 'theme-preview--dark' },
  { id: 'light',  label: 'Light',  previewClass: 'theme-preview--light' },
  { id: 'dimmed', label: 'Dimmed', previewClass: 'theme-preview--dimmed' },
];

export function SettingsPanel() {
  const { state, dispatch } = useApp();
  const tauri = useTauri();
  const [testing, setTesting] = useState(false);
  const [urlDraft, setUrlDraft] = useState(state.llmUrl);

  async function testConnection() {
    setTesting(true);
    try {
      const ok = await tauri.checkServer(urlDraft);
      dispatch({ type: 'SET_LLM_URL', url: urlDraft });
      if (ok) {
        const model = await tauri.getModelName(urlDraft);
        dispatch({ type: 'SET_SERVER_STATUS', online: true, model });
      } else {
        dispatch({ type: 'SET_SERVER_STATUS', online: false });
      }
    } finally {
      setTesting(false);
    }
  }

  return (
    <>
      <div className="sidebar__header">SETTINGS</div>
      <div className="sidebar__scroll" style={{ padding: '12px 12px' }}>

        {/* Theme */}
        <div style={{ marginBottom: 20 }}>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 8 }}>
            Theme
          </div>
          <div className="theme-switcher">
            {THEMES.map(t => (
              <div
                key={t.id}
                className={`theme-option ${state.theme === t.id ? 'theme-option--active' : ''}`}
                onClick={() => dispatch({ type: 'SET_THEME', theme: t.id })}
              >
                <div className={`theme-preview ${t.previewClass}`} />
                <span>{t.label}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Server */}
        <div style={{ marginBottom: 20 }}>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 8 }}>
            LLM Server
          </div>
          <div style={{ display: 'flex', gap: 6, alignItems: 'center', marginBottom: 6 }}>
            <div style={{ width: 8, height: 8, borderRadius: '50%', background: state.serverOnline ? 'var(--status-ok)' : 'var(--status-err)', flexShrink: 0 }} />
            <span style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
              {state.serverOnline ? `Connected · ${state.modelName || 'unknown'}` : 'Not connected'}
            </span>
          </div>
          <input
            className="input"
            value={urlDraft}
            onChange={e => setUrlDraft(e.target.value)}
            placeholder="http://127.0.0.1:8080"
            style={{ marginBottom: 6 }}
          />
          <button
            className="btn btn--secondary"
            style={{ width: '100%', justifyContent: 'center' }}
            onClick={testConnection}
            disabled={testing}
          >
            {testing ? <><span className="spinner" style={{ width: 12, height: 12 }} /> Testing…</> : 'Test Connection'}
          </button>
        </div>

        {/* Keyboard shortcuts */}
        <div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 8 }}>
            Shortcuts
          </div>
          {[
            ['Send message', 'Enter'],
            ['New line', 'Shift+Enter'],
            ['New session', 'Ctrl+N'],
            ['Toggle sidebar', 'Ctrl+B'],
          ].map(([action, key]) => (
            <div key={action} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6, fontSize: 12 }}>
              <span style={{ color: 'var(--text-secondary)' }}>{action}</span>
              <kbd style={{ fontFamily: 'var(--font-mono)', fontSize: 11, background: 'var(--bg-active)', border: '1px solid var(--border)', borderRadius: 3, padding: '1px 5px', color: 'var(--text-primary)' }}>{key}</kbd>
            </div>
          ))}
        </div>

      </div>
    </>
  );
}
