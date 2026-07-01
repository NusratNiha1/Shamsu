import { useApp } from '../../store/AppContext';
import type { Session } from '../../store/appStore';
import { nanoid } from '../../lib/nanoid';

export function SessionsPanel() {
  const { state, dispatch } = useApp();

  function newSession() {
    const name = `Session ${state.sessions.length + 1}`;
    const session: Session = {
      id: nanoid(),
      name,
      workspace: '',
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    dispatch({ type: 'CREATE_SESSION', session });
  }

  return (
    <>
      <div className="sidebar__header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span>SESSIONS</span>
        <button className="btn btn--ghost btn--icon" onClick={newSession} title="New">+</button>
      </div>
      <div className="sidebar__scroll">
        {state.sessions.map(s => (
          <div
            key={s.id}
            className={`session-item ${s.id === state.activeSessionId ? 'session-item--active' : ''}`}
            onClick={() => dispatch({ type: 'SET_ACTIVE_SESSION', id: s.id })}
          >
            <div className="session-item__dot" />
            <div className="session-item__name">{s.name}</div>
            <button
              className="btn btn--ghost btn--icon"
              style={{ fontSize: 11, marginLeft: 'auto' }}
              onClick={e => { e.stopPropagation(); dispatch({ type: 'DELETE_SESSION', id: s.id }); }}
              title="Delete"
            >✕</button>
          </div>
        ))}
      </div>
    </>
  );
}
