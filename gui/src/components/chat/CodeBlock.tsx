import { useState } from 'react';

interface Props {
  lang: string;
  code: string;
}

export function CodeBlock({ lang, code }: Props) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="message-code-block">
      <div className="message-code-block__header">
        <span>{lang || 'code'}</span>
        <button className="message-code-block__copy" onClick={copy}>
          {copied ? '✓ copied' : 'copy'}
        </button>
      </div>
      <pre><code>{code}</code></pre>
    </div>
  );
}
