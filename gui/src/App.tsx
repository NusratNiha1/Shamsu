import { useEffect } from 'react';
import { useApp } from './store/AppContext';
import { useServerPoll } from './hooks/useServerPoll';
import { TitleBar } from './components/TitleBar';
import { ActivityBar } from './components/ActivityBar';
import { Sidebar } from './components/Sidebar';
import { MainArea } from './components/MainArea';
import { StatusBar } from './components/StatusBar';

function AppShell() {
  const { dispatch } = useApp();
  useServerPoll();

  // Global keyboard shortcuts
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault();
        dispatch({ type: 'TOGGLE_SIDEBAR' });
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dispatch]);

  return (
    <div className="app-shell">
      <TitleBar />
      <div className="body-row">
        <ActivityBar />
        <Sidebar />
        <MainArea />
      </div>
      <StatusBar />
    </div>
  );
}

export default function App() {
  return <AppShell />;
}
