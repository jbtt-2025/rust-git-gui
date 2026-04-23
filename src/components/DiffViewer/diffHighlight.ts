import { createHighlighter, type Highlighter } from 'shiki';

let highlighterPromise: Promise<Highlighter> | null = null;

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: ['github-dark', 'github-light'],
      langs: [
        'javascript', 'typescript', 'tsx', 'jsx', 'json', 'html', 'css',
        'python', 'rust', 'go', 'java', 'c', 'cpp', 'markdown', 'yaml',
        'toml', 'shell', 'sql', 'xml', 'ruby', 'php', 'swift', 'kotlin',
      ],
    });
  }
  return highlighterPromise;
}

/** Infer language from file extension */
function langFromPath(path: string): string | undefined {
  const ext = path.split('.').pop()?.toLowerCase();
  const map: Record<string, string> = {
    js: 'javascript', ts: 'typescript', tsx: 'tsx', jsx: 'jsx',
    json: 'json', html: 'html', css: 'css', py: 'python',
    rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp',
    h: 'c', hpp: 'cpp', md: 'markdown', yml: 'yaml', yaml: 'yaml',
    toml: 'toml', sh: 'shell', bash: 'shell', sql: 'sql',
    xml: 'xml', rb: 'ruby', php: 'php', swift: 'swift', kt: 'kotlin',
  };
  return ext ? map[ext] : undefined;
}

/**
 * Highlight a code string using Shiki. Returns HTML tokens per line.
 * Falls back to plain text if language is unsupported.
 */
export async function highlightCode(
  code: string,
  filePath: string,
  theme: 'github-dark' | 'github-light' = 'github-dark',
): Promise<string[]> {
  const lang = langFromPath(filePath);
  if (!lang) {
    return code.split('\n').map((line) => escapeHtml(line));
  }

  try {
    const hl = await getHighlighter();
    const result = hl.codeToHtml(code, { lang, theme });
    // Extract inner lines from the HTML
    const lines = extractLinesFromHtml(result);
    return lines;
  } catch {
    return code.split('\n').map((line) => escapeHtml(line));
  }
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

/** Extract individual line HTML from Shiki output */
function extractLinesFromHtml(html: string): string[] {
  // Shiki wraps lines in <span class="line">...</span>
  const lineRegex = /<span class="line">(.*?)<\/span>/g;
  const lines: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = lineRegex.exec(html)) !== null) {
    lines.push(match[1]);
  }
  return lines.length > 0 ? lines : [html];
}
