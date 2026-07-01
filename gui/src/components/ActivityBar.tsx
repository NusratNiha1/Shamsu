import { useApp } from '../store/AppContext';
import type { SidebarView } from '../store/appStore';

const ICONS: { id: SidebarView; label: string; icon: string }[] = [
  { id: 'chat',     label: 'Chat',     icon: '💬' },
  { id: 'explorer', label: 'Explorer', icon: '📁' },
  { id: 'sessions', label: 'Sessions', icon: '🗂' },
  { id: 'settings', label: 'Settings', icon: '⚙️' },
];

export function ActivityBar() {
  const { state, dispatch } = useApp();

  function handleClick(view: SidebarView) {
    if (state.sidebarView === view && state.sidebarOpen) {
      dispatch({ type: 'TOGGLE_SIDEBAR' });
    } else {
      dispatch({ type: 'SET_SIDEBAR_VIEW', view });
    }
  }

  return (
    <div className="activity-bar">
      {ICONS.map(({ id, label, icon }) => (
        <div
          key={id}
          className={`activity-bar__icon ${state.sidebarView === id && state.sidebarOpen ? 'activity-bar__icon--active' : ''}`}
          onClick={() => handleClick(id)}
          data-tooltip={label}
          title={label}
        >
          <span style={{ fontSize: 16 }}>{icon}</span>
        </div>
      ))}
    </div>
  );
}
