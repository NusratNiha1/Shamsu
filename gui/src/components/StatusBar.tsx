import { useApp } from '../store/AppContext';

export function StatusBar() {
  const { state, dispatch } = useApp();
  const session = state.sessions.find(s => s.id === state.activeSessionId);

  return (
    <div className={`statusbar ${state.serverOnline ? '' : 'statusbar--disconnected'}`}>
      <div className="statusbar__item" onClick={() => dispatch({ type: 'SET_SIDEBAR_VIEW', view: 'settings' })}>
        {state.serverOnline ? '◆' : '⚠'} {state.serverOnline ? 'connected' : 'server offline'}
      </div>

      {state.modelName && (
        <div className="statusbar__item">
          {state.modelName.split('/').pop()?.split('-').slice(0, 3).join('-')}
        </div>
      )}

      <div className="statusbar__spacer" />

      {session && (
        <div className="statusbar__item">
          {session.name}
        </div>
      )}

      {state.activeFile && (
        <div className="statusbar__item" style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>
          {state.activeFile.split(/[/\\]/).pop()}
        </div>
      )}

      <div
        className="statusbar__item"
        onClick={() => dispatch({ type: 'SET_SIDEBAR_VIEW', view: 'settings' })}
        title="Change theme"
      >
        {{dark: '◑', light: '○', dimmed: '●'}[state.theme]}
      </div>
    </div>
  );
}
