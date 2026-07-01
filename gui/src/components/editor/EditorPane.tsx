import { useApp } from '../../store/AppContext';

export function EditorPane() {
  const { state, dispatch } = useApp();

  if (!state.activeFile) {
    return (
      <div className="editor-area" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div className="empty-state">
          <div className="empty-state__icon">📄</div>
          <div className="empty-state__hint">Open a file from the explorer or ask Shamsu to create one</div>
        </div>
      </div>
    );
  }

  const content = state.fileContents[state.activeFile] ?? '';

  return (
    <div className="editor-area">
      <textarea
        className="selectable"
        value={content}
        onChange={e => dispatch({ type: 'UPDATE_FILE_CONTENT', path: state.activeFile!, content: e.target.value })}
        style={{
          width: '100%',
          height: '100%',
          background: 'var(--bg-base)',
          color: 'var(--code-text)',
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-sm)',
          lineHeight: 'var(--leading-relaxed)',
          padding: '16px 20px',
          resize: 'none',
          border: 'none',
          outline: 'none',
          overflowY: 'auto',
          boxSizing: 'border-box',
        }}
        spellCheck={false}
      />
    </div>
  );
}
