import { useApp } from '../store/AppContext';

export function TitleBar() {
  const { state } = useApp();

  async function handleClose() {
    if (window.__TAURI__) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      getCurrentWindow().close();
    }
  }
  async function handleMin() {
    if (window.__TAURI__) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      getCurrentWindow().minimize();
    }
  }
  async function handleMax() {
    if (window.__TAURI__) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      getCurrentWindow().toggleMaximize();
    }
  }

  return (
    <div className="titlebar">
      <div className="titlebar__traffic-lights">
        <div className="titlebar__dot titlebar__dot--close" onClick={handleClose} title="Close" />
        <div className="titlebar__dot titlebar__dot--min"   onClick={handleMin}   title="Minimize" />
        <div className="titlebar__dot titlebar__dot--max"   onClick={handleMax}   title="Maximize" />
      </div>

      <span className="titlebar__logo">SHAMSU</span>

      <div className="titlebar__spacer" />

      <div className="titlebar__actions">
        {!state.serverOnline && (
          <span className="badge badge--err" style={{ fontSize: 11 }}>⚠ Server offline</span>
        )}
        {state.serverOnline && state.modelName && (
          <span className="badge" style={{ fontSize: 11 }}>
            ◆ {state.modelName.split('/').pop()?.split('-').slice(0,3).join('-')}
          </span>
        )}
      </div>
    </div>
  );
}
