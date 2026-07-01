// Agent hook — orchestrates LLM call + code extraction + file writing
// This mirrors what the Rust CLI does, but in TypeScript for the GUI.

import { useCallback, useRef } from 'react';
import { useApp } from '../store/AppContext';
import { useTauri } from './useTauri';
import type { ChatMessage } from '../store/appStore';
import { extractCodeBlocks } from '../lib/extractor';
import type { CodeBlock } from '../lib/extractor';
import { nanoid } from '../lib/nanoid';

const COOKING_PHASES = [
  ['🍳', 'Cracking eggs…'],
  ['🔥', 'Heating things up…'],
  ['✨', 'Conjuring some magic…'],
  ['🧙', 'Casting spells…'],
  ['⚗️', 'Brewing the solution…'],
  ['🍵', 'Steeping the logic…'],
  ['🔮', 'Reading the crystal ball…'],
  ['🎨', 'Painting pixels…'],
  ['🧩', 'Fitting the pieces…'],
  ['🚀', 'Preparing for launch…'],
] as const;

const SYSTEM_PROMPT = `You are Shamsu, an offline-first AI developer assistant running locally.

When writing code:
- Put each file in its own fenced code block with the language tag
- On the line immediately before the code block, write the filename like: \`filename.ext\`
- Include ALL files needed for the feature
- Be concise in explanations`;

export function useAgent() {
  const { state, dispatch } = useApp();
  const tauri = useTauri();
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const secondsRef = useRef(0);

  function startCooking() {
    secondsRef.current = 0;
    dispatch({ type: 'SET_GENERATING', generating: true });
    dispatch({ type: 'SET_COOKING', phase: `${COOKING_PHASES[0][0]} ${COOKING_PHASES[0][1]}`, seconds: 0 });

    timerRef.current = setInterval(() => {
      secondsRef.current += 1;
      const phaseIdx = Math.floor(secondsRef.current / 4) % COOKING_PHASES.length;
      const [icon, msg] = COOKING_PHASES[phaseIdx];
      dispatch({
        type: 'SET_COOKING',
        phase: `${icon} ${msg}`,
        seconds: secondsRef.current,
      });
    }, 1000);
  }

  function stopCooking() {
    if (timerRef.current) clearInterval(timerRef.current);
    timerRef.current = null;
    dispatch({ type: 'SET_GENERATING', generating: false });
    dispatch({ type: 'SET_COOKING', phase: '', seconds: 0 });
  }

  const sendMessage = useCallback(async (userText: string) => {
    const sessionId = state.activeSessionId;
    if (!sessionId || !userText.trim()) return;

    const session = state.sessions.find(s => s.id === sessionId);
    if (!session) return;

    // Add user message
    const userMsg: ChatMessage = {
      id: nanoid(),
      role: 'user',
      content: userText.trim(),
      timestamp: Date.now(),
    };
    dispatch({ type: 'APPEND_MESSAGE', sessionId, message: userMsg });

    // Add typing indicator
    const typingMsg: ChatMessage = {
      id: nanoid(),
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
      isTyping: true,
    };
    dispatch({ type: 'APPEND_MESSAGE', sessionId, message: typingMsg });

    startCooking();

    try {
      // Build message history for LLM
      const history: ChatMessage[] = [
        { id: 'sys', role: 'system', content: SYSTEM_PROMPT, timestamp: 0 },
        ...session.messages.filter(m => !m.isTyping).slice(-20),
        userMsg,
      ];

      const response = await tauri.sendMessage(history, state.llmUrl, 0.7, 2048);

      // Extract code blocks and write files
      const blocks: CodeBlock[] = extractCodeBlocks(response);
      const writtenFiles: string[] = [];

      for (const block of blocks) {
        if (block.filename) {
          try {
            const fullPath = session.workspace
              ? `${session.workspace}/${block.filename}`
              : block.filename;
            await tauri.writeFile(fullPath, block.content);
            writtenFiles.push(block.filename);

            // Open in editor tab
            dispatch({ type: 'OPEN_FILE', path: fullPath, content: block.content });
          } catch (e) {
            console.error('writeFile failed:', e);
          }
        }
      }

      stopCooking();

      // Replace typing indicator with real response
      dispatch({
        type: 'UPDATE_LAST_MESSAGE',
        sessionId,
        content: response,
        files: writtenFiles.length > 0 ? writtenFiles : undefined,
      });

    } catch (err) {
      stopCooking();
      dispatch({
        type: 'UPDATE_LAST_MESSAGE',
        sessionId,
        content: `Error: ${err instanceof Error ? err.message : String(err)}`,
      });
    }
  }, [state.activeSessionId, state.sessions, state.llmUrl, dispatch]);

  return { sendMessage, isGenerating: state.isGenerating };
}
