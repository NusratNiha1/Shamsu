import { useApp } from '../../store/AppContext';

export function TabBar() {
  const { state, dispatch } = useApp();

  if (state.openFiles.length === 0) return null;

  return (
    <div className="tab-bar">
      {state.openFiles.map(path => {
        const name = path.split(/[/\\]/).pop() ?? path;
        const isActive = path === state.activeFile;
        return (
          <div
            key={path}
            className={`tab ${isActive ? 'tab--active' : ''}`}
            onClick={() => dispatch({ type: 'SET_ACTIVE_FILE', path })}
            title={path}
          >
            <span style={{ fontSize: 12 }}>📄</span>
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>{name}</span>
            <button
              className="tab__close"
              onClick={e => { e.stopPropagation(); dispatch({ type: 'CLOSE_FILE', path }); }}
              title="Close"
            >✕</button>
          </div>
        );
      })}
    </div>
  );
}
