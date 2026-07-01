import { useEffect } from 'react';
import { useApp } from '../store/AppContext';
import { useTauri } from './useTauri';

/** Polls the llama.cpp server every 8 seconds and updates serverOnline/modelName. */
export function useServerPoll() {
  const { state, dispatch } = useApp();
  const tauri = useTauri();

  useEffect(() => {
    let cancelled = false;

    async function poll() {
      if (cancelled) return;
      try {
        const ok = await tauri.checkServer(state.llmUrl);
        if (cancelled) return;
        if (ok) {
          const model = await tauri.getModelName(state.llmUrl);
          if (!cancelled) dispatch({ type: 'SET_SERVER_STATUS', online: true, model });
        } else {
          if (!cancelled) dispatch({ type: 'SET_SERVER_STATUS', online: false });
        }
      } catch {
        if (!cancelled) dispatch({ type: 'SET_SERVER_STATUS', online: false });
      }
    }

    poll();
    const id = setInterval(poll, 8000);
    return () => { cancelled = true; clearInterval(id); };
  }, [state.llmUrl]);
}
