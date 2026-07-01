import { useApp } from '../../store/AppContext';
import type { Session } from '../../store/appStore';
import { nanoid } from '../../lib/nanoid';

export function ChatSessionList() {
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

  function relativeTime(ts: number): string {
    const diff = Date.now() - ts;
    if (diff < 60_000)  return 'just now';
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return `${Math.floor(diff / 86_400_000)}d ago`;
  }

  return (
    <>
      <div className="sidebar__header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span>CHATS</span>
        <button className="btn btn--ghost btn--icon" onClick={newSession} title="New session" style={{ fontSize: 16 }}>+</button>
      </div>
      <div className="sidebar__scroll">
        {state.sessions.length === 0 && (
          <div className="empty-state" style={{ padding: 24 }}>
            <div className="empty-state__icon">💬</div>
            <div className="empty-state__hint">No sessions yet</div>
            <button className="btn btn--secondary" onClick={newSession}>New Chat</button>
          </div>
        )}
        {state.sessions.map(s => (
          <div
            key={s.id}
            className={`session-item ${s.id === state.activeSessionId ? 'session-item--active' : ''}`}
            onClick={() => dispatch({ type: 'SET_ACTIVE_SESSION', id: s.id })}
          >
            <div className="session-item__dot" />
            <div className="session-item__name">{s.name}</div>
            <div className="session-item__time">{relativeTime(s.updatedAt)}</div>
          </div>
        ))}
      </div>
    </>
  );
}
