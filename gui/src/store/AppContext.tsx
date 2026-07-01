import { createContext, useContext, useReducer, useEffect, type ReactNode } from 'react';
import { type AppState, type AppAction, appReducer, initialState } from './appStore';

interface AppContextValue {
  state: AppState;
  dispatch: React.Dispatch<AppAction>;
}

const AppContext = createContext<AppContextValue | null>(null);

const STORAGE_KEY = 'shamsu_state_v1';

function loadPersistedState(): Partial<AppState> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    // Only restore safe keys
    return {
      theme: parsed.theme,
      llmUrl: parsed.llmUrl,
      sessions: parsed.sessions ?? [],
      activeSessionId: parsed.activeSessionId,
    };
  } catch {
    return {};
  }
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(appReducer, {
    ...initialState,
    ...loadPersistedState(),
  });

  // Persist key state to localStorage
  useEffect(() => {
    const toSave = {
      theme: state.theme,
      llmUrl: state.llmUrl,
      sessions: state.sessions,
      activeSessionId: state.activeSessionId,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(toSave));
  }, [state.theme, state.llmUrl, state.sessions, state.activeSessionId]);

  // Apply theme to DOM
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', state.theme);
  }, [state.theme]);

  return (
    <AppContext.Provider value={{ state, dispatch }}>
      {children}
    </AppContext.Provider>
  );
}

export function useApp() {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error('useApp must be used inside AppProvider');
  return ctx;
}
