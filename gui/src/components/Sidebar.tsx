import { useApp } from '../store/AppContext';
import { SessionsPanel } from './panels/SessionsPanel';
import { ExplorerPanel } from './panels/ExplorerPanel';
import { SettingsPanel } from './panels/SettingsPanel';
import { ChatSessionList } from './panels/ChatSessionList';

export function Sidebar() {
  const { state } = useApp();

  if (!state.sidebarOpen) return null;

  return (
    <div className="sidebar">
      {state.sidebarView === 'chat'     && <ChatSessionList />}
      {state.sidebarView === 'explorer' && <ExplorerPanel />}
      {state.sidebarView === 'sessions' && <SessionsPanel />}
      {state.sidebarView === 'settings' && <SettingsPanel />}
    </div>
  );
}
