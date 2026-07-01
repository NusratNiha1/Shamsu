import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './design-system/index.css';
import { AppProvider } from './store/AppContext';
import App from './App';

// Set default theme before render to avoid flash
const saved = localStorage.getItem('shamsu_state_v1');
const theme = saved ? (JSON.parse(saved).theme ?? 'dark') : 'dark';
document.documentElement.setAttribute('data-theme', theme);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AppProvider>
      <App />
    </AppProvider>
  </StrictMode>
);
