// Central app state — no external library needed, just React context + useReducer

export type Theme = 'dark' | 'light' | 'dimmed';

export type SidebarView = 'chat' | 'explorer' | 'sessions' | 'settings';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  files?: string[];   // filenames written during this turn
  isTyping?: boolean; // cooking indicator
}

export interface Session {
  id: string;
  name: string;
  workspace: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

export interface AppState {
  theme: Theme;
  sidebarView: SidebarView;
  sidebarOpen: boolean;
  sessions: Session[];
  activeSessionId: string | null;
  llmUrl: string;
  serverOnline: boolean;
  modelName: string;
  isGenerating: boolean;
  cookingPhase: string;
  cookingSeconds: number;
  openFiles: string[];         // tabs
  activeFile: string | null;
  fileContents: Record<string, string>;
}

export type AppAction =
  | { type: 'SET_THEME'; theme: Theme }
  | { type: 'SET_SIDEBAR_VIEW'; view: SidebarView }
  | { type: 'TOGGLE_SIDEBAR' }
  | { type: 'SET_SERVER_STATUS'; online: boolean; model?: string }
  | { type: 'CREATE_SESSION'; session: Session }
  | { type: 'SET_ACTIVE_SESSION'; id: string }
  | { type: 'DELETE_SESSION'; id: string }
  | { type: 'APPEND_MESSAGE'; sessionId: string; message: ChatMessage }
  | { type: 'UPDATE_LAST_MESSAGE'; sessionId: string; content: string; files?: string[] }
  | { type: 'REMOVE_TYPING'; sessionId: string }
  | { type: 'SET_GENERATING'; generating: boolean }
  | { type: 'SET_COOKING'; phase: string; seconds: number }
  | { type: 'OPEN_FILE'; path: string; content: string }
  | { type: 'CLOSE_FILE'; path: string }
  | { type: 'SET_ACTIVE_FILE'; path: string }
  | { type: 'UPDATE_FILE_CONTENT'; path: string; content: string }
  | { type: 'SET_LLM_URL'; url: string };

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case 'SET_THEME':
      return { ...state, theme: action.theme };

    case 'SET_SIDEBAR_VIEW':
      return { ...state, sidebarView: action.view, sidebarOpen: true };

    case 'TOGGLE_SIDEBAR':
      return { ...state, sidebarOpen: !state.sidebarOpen };

    case 'SET_SERVER_STATUS':
      return { ...state, serverOnline: action.online, modelName: action.model ?? state.modelName };

    case 'CREATE_SESSION':
      return { ...state, sessions: [action.session, ...state.sessions], activeSessionId: action.session.id };

    case 'SET_ACTIVE_SESSION':
      return { ...state, activeSessionId: action.id };

    case 'DELETE_SESSION':
      return {
        ...state,
        sessions: state.sessions.filter(s => s.id !== action.id),
        activeSessionId: state.activeSessionId === action.id
          ? (state.sessions.find(s => s.id !== action.id)?.id ?? null)
          : state.activeSessionId,
      };

    case 'APPEND_MESSAGE': {
      const sessions = state.sessions.map(s =>
        s.id === action.sessionId
          ? { ...s, messages: [...s.messages, action.message], updatedAt: Date.now() }
          : s
      );
      return { ...state, sessions };
    }

    case 'UPDATE_LAST_MESSAGE': {
      const sessions = state.sessions.map(s => {
        if (s.id !== action.sessionId) return s;
        const messages = [...s.messages];
        const last = messages[messages.length - 1];
        if (last && last.isTyping) {
          messages[messages.length - 1] = {
            ...last,
            content: action.content,
            files: action.files,
            isTyping: false,
          };
        }
        return { ...s, messages, updatedAt: Date.now() };
      });
      return { ...state, sessions };
    }

    case 'REMOVE_TYPING': {
      const sessions = state.sessions.map(s =>
        s.id === action.sessionId
          ? { ...s, messages: s.messages.filter(m => !m.isTyping) }
          : s
      );
      return { ...state, sessions };
    }

    case 'SET_GENERATING':
      return { ...state, isGenerating: action.generating };

    case 'SET_COOKING':
      return { ...state, cookingPhase: action.phase, cookingSeconds: action.seconds };

    case 'OPEN_FILE': {
      const openFiles = state.openFiles.includes(action.path)
        ? state.openFiles
        : [...state.openFiles, action.path];
      return {
        ...state,
        openFiles,
        activeFile: action.path,
        fileContents: { ...state.fileContents, [action.path]: action.content },
      };
    }

    case 'CLOSE_FILE': {
      const openFiles = state.openFiles.filter(f => f !== action.path);
      return {
        ...state,
        openFiles,
        activeFile: state.activeFile === action.path
          ? (openFiles[openFiles.length - 1] ?? null)
          : state.activeFile,
      };
    }

    case 'SET_ACTIVE_FILE':
      return { ...state, activeFile: action.path };

    case 'UPDATE_FILE_CONTENT':
      return { ...state, fileContents: { ...state.fileContents, [action.path]: action.content } };

    case 'SET_LLM_URL':
      return { ...state, llmUrl: action.url };

    default:
      return state;
  }
}

export const initialState: AppState = {
  theme: 'dark',
  sidebarView: 'chat',
  sidebarOpen: true,
  sessions: [],
  activeSessionId: null,
  llmUrl: 'http://127.0.0.1:8080',
  serverOnline: false,
  modelName: '',
  isGenerating: false,
  cookingPhase: '',
  cookingSeconds: 0,
  openFiles: [],
  activeFile: null,
  fileContents: {},
};
