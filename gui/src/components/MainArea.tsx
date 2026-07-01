import { useState } from 'react';
import { useApp } from '../store/AppContext';
import { ChatPanel } from './chat/ChatPanel';
import { TabBar } from './editor/TabBar';
import { EditorPane } from './editor/EditorPane';

export function MainArea() {
  const { state } = useApp();
  // Split ratio: fraction of height given to chat (0.0 = all editor, 1.0 = all chat)
  const [chatRatio, setChatRatio] = useState(0.55);
  const hasOpenFiles = state.openFiles.length > 0;

  function onDrag(e: React.MouseEvent<HTMLDivElement>) {
    const container = (e.currentTarget as HTMLElement).parentElement;
    if (!container) return;
    const startY = e.clientY;
    const startRatio = chatRatio;
    const totalH = container.getBoundingClientRect().height;

    function onMove(me: MouseEvent) {
      const delta = (me.clientY - startY) / totalH;
      setChatRatio(Math.max(0.2, Math.min(0.85, startRatio + delta)));
    }
    function onUp() {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  return (
    <div className="main-area">
      {/* Chat panel — always visible, fills space when no files open */}
      <div style={{
        height: hasOpenFiles ? `${chatRatio * 100}%` : '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        transition: hasOpenFiles ? 'none' : 'height 0.2s ease',
      }}>
        <ChatPanel />
      </div>

      {/* Resizable divider + editor — only when files are open */}
      {hasOpenFiles && (
        <>
          {/* Drag handle */}
          <div
            style={{
              height: 4,
              background: 'var(--border)',
              cursor: 'row-resize',
              flexShrink: 0,
              transition: 'background var(--t-fast)',
            }}
            onMouseDown={onDrag}
            onMouseEnter={e => (e.currentTarget.style.background = 'var(--border-strong)')}
            onMouseLeave={e => (e.currentTarget.style.background = 'var(--border)')}
          />
          {/* Editor */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <TabBar />
            <EditorPane />
          </div>
        </>
      )}
    </div>
  );
}
