import type { ChatMessage } from '../../store/appStore';
import { CodeBlock } from './CodeBlock';

interface Props {
  message: ChatMessage;
}

// Render markdown-ish content: split on ``` fences, render code blocks separately
function renderContent(content: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  const lines = content.split('\n');
  let inCode = false;
  let codeLang = '';
  let codeLines: string[] = [];
  let textLines: string[] = [];
  let key = 0;

  function flushText() {
    if (!textLines.length) return;
    // Render inline code inside text
    const text = textLines.join('\n').trim();
    if (text) {
      parts.push(
        <p key={key++} style={{ margin: '2px 0', whiteSpace: 'pre-wrap' }}>
          {renderInline(text)}
        </p>
      );
    }
    textLines = [];
  }

  function flushCode() {
    if (!codeLines.length) return;
    parts.push(
      <CodeBlock key={key++} lang={codeLang} code={codeLines.join('\n')} />
    );
    codeLines = [];
    codeLang = '';
  }

  for (const line of lines) {
    const trimmed = line.trim();
    if (!inCode && trimmed.startsWith('```')) {
      flushText();
      inCode = true;
      codeLang = trimmed.slice(3).trim();
    } else if (inCode && trimmed === '```') {
      flushCode();
      inCode = false;
    } else if (inCode) {
      codeLines.push(line);
    } else {
      textLines.push(line);
    }
  }

  if (inCode) flushCode();
  flushText();

  return parts;
}

function renderInline(text: string): React.ReactNode {
  // Simple inline code rendering: `code`
  const parts: React.ReactNode[] = [];
  const regex = /`([^`]+)`/g;
  let last = 0;
  let match;
  let i = 0;
  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) parts.push(<span key={i++}>{text.slice(last, match.index)}</span>);
    parts.push(<code key={i++}>{match[1]}</code>);
    last = match.index + match[0].length;
  }
  if (last < text.length) parts.push(<span key={i++}>{text.slice(last)}</span>);
  return <>{parts}</>;
}

export function MessageRow({ message }: Props) {
  const isUser = message.role === 'user';

  return (
    <div className={`message-row ${isUser ? 'message-row--user' : ''}`}>
      <div className={`message-avatar ${isUser ? '' : 'message-avatar--ai'}`}>
        {isUser ? 'U' : 'S'}
      </div>

      <div className="message-bubble">
        {renderContent(message.content)}

        {/* File written badges */}
        {message.files && message.files.length > 0 && (
          <div style={{ marginTop: 8, display: 'flex', flexWrap: 'wrap', gap: 4 }}>
            {message.files.map(f => (
              <span key={f} className="file-written-badge">
                ✓ {f.split('/').pop()}
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
