import { useApp } from '../../store/AppContext';
import { useTauri } from '../../hooks/useTauri';
import { useState } from 'react';

export function ExplorerPanel() {
  const { state, dispatch } = useApp();
  const tauri = useTauri();
  const [entries, setEntries] = useState<string[]>([]);
  const [currentPath, setCurrentPath] = useState('');

  const session = state.sessions.find(s => s.id === state.activeSessionId);
  const workspace = session?.workspace ?? '';

  async function loadDir(path: string) {
    try {
      const list = await tauri.listDir(path);
      setEntries(list);
      setCurrentPath(path);
    } catch {
      setEntries([]);
    }
  }

  async function openFile(entry: string) {
    const fullPath = `${currentPath}/${entry}`;
    try {
      const content = await tauri.readFile(fullPath);
      dispatch({ type: 'OPEN_FILE', path: fullPath, content });
    } catch {}
  }

  return (
    <>
      <div className="sidebar__header">EXPLORER</div>
      <div className="sidebar__scroll">
        {!workspace && (
          <div className="empty-state" style={{ padding: 16 }}>
            <div className="empty-state__icon">📁</div>
            <div className="empty-state__hint">No workspace open</div>
          </div>
        )}
        {workspace && entries.length === 0 && (
          <div style={{ padding: 12 }}>
            <button className="btn btn--secondary" style={{ width: '100%', fontSize: 12 }} onClick={() => loadDir(workspace)}>
              Load workspace
            </button>
          </div>
        )}
        {entries.map(entry => (
          <div
            key={entry}
            className={`tree-item ${state.activeFile?.endsWith(entry) ? 'tree-item--active' : ''}`}
            onClick={() => entry.endsWith('/') ? loadDir(`${currentPath}/${entry.slice(0,-1)}`) : openFile(entry)}
          >
            <span className="tree-item__icon">{entry.endsWith('/') ? '📁' : '📄'}</span>
            <span className="tree-item__name">{entry}</span>
          </div>
        ))}
      </div>
    </>
  );
}
