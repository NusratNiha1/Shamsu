// Code block extractor — same logic as the Rust extractor.rs but in TypeScript.

export interface CodeBlock {
  lang: string;
  filename: string | null;
  content: string;
}

const KNOWN_EXTS = new Set([
  'rs','py','js','ts','jsx','tsx','go','java','c','cpp','h','hpp',
  'cs','rb','php','swift','kt','md','toml','yaml','yml','json',
  'html','css','scss','sass','sh','bash','sql','txt','env','xml',
  'vue','svelte','lock','conf','cfg','ini',
]);

const LANG_DEFAULTS: Record<string, string> = {
  html: 'index.html',
  css: 'styles.css',
  javascript: 'script.js',
  js: 'script.js',
  typescript: 'index.ts',
  ts: 'index.ts',
  jsx: 'index.jsx',
  tsx: 'index.tsx',
  python: 'main.py',
  py: 'main.py',
  rust: 'main.rs',
  rs: 'main.rs',
  go: 'main.go',
  java: 'Main.java',
  json: 'config.json',
  toml: 'Cargo.toml',
  yaml: 'config.yaml',
  yml: 'config.yml',
  sh: 'run.sh',
  bash: 'run.sh',
  sql: 'schema.sql',
  markdown: 'README.md',
  md: 'README.md',
};

function looksLikeFilename(s: string): boolean {
  if (!s || s.length > 120 || s.includes(' ')) return false;
  const dot = s.lastIndexOf('.');
  if (dot < 0) return false;
  return KNOWN_EXTS.has(s.slice(dot + 1));
}

function findFilenameBeforeFence(lines: string[], fenceIdx: number): string | null {
  const start = Math.max(0, fenceIdx - 6);
  for (let i = fenceIdx - 1; i >= start; i--) {
    const line = lines[i].trim();
    if (!line) continue;

    // 1. Backtick-wrapped: `filename.ext`
    const btMatches = [...line.matchAll(/`([^`]+)`/g)];
    for (const m of btMatches) {
      if (looksLikeFilename(m[1])) return m[1];
    }

    // 2. "named X"
    const namedMatch = line.match(/named\s+[`'"]?(\S+?)[`'"]?[,\s]?$/i);
    if (namedMatch && looksLikeFilename(namedMatch[1])) return namedMatch[1];

    // 3. Bold **filename**
    const boldMatch = line.match(/\*\*([^*]+)\*\*/);
    if (boldMatch && looksLikeFilename(boldMatch[1])) return boldMatch[1];

    // 4. Comment prefix: // filename  or  # filename
    const commentMatch = line.match(/^(?:\/\/|#)\s+(\S+)$/);
    if (commentMatch && looksLikeFilename(commentMatch[1])) return commentMatch[1];

    // 5. Bare filename/path on its own line
    const bare = line.replace(/:$/, '').trim();
    if (looksLikeFilename(bare)) return bare;
    if ((bare.includes('/') || bare.includes('\\')) && looksLikeFilename(bare.split(/[/\\]/).pop() ?? '')) {
      return bare;
    }
  }
  return null;
}

export function extractCodeBlocks(response: string): CodeBlock[] {
  const blocks: CodeBlock[] = [];
  const lines = response.split('\n');
  let i = 0;

  while (i < lines.length) {
    const trimmed = lines[i].trim();

    if (!trimmed.startsWith('```')) { i++; continue; }

    const lang = trimmed.slice(3).trim().toLowerCase();

    // Skip tool_call blocks
    if (lang === 'tool_call') { i++; continue; }

    // Find closing ```
    let close = i + 1;
    while (close < lines.length && lines[close].trim() !== '```') close++;

    const codeLines = lines.slice(i + 1, close);
    const content = codeLines.join('\n');

    if (content.trim()) {
      // Shell blocks: don't write to files
      if (!['bash','sh','shell','powershell','ps1','cmd','bat'].includes(lang)) {
        const filename = findFilenameBeforeFence(lines, i) ?? LANG_DEFAULTS[lang] ?? null;
        blocks.push({ lang, filename, content: content + '\n' });
      }
    }

    i = close + 1;
  }

  return blocks;
}
