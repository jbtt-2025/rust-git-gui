import { useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Command } from '@tauri-apps/plugin-shell';
import { useUiStore } from '../../stores/uiStore';
import { useRepoStore } from '../../stores/repoStore';
import '@xterm/xterm/css/xterm.css';
import './TerminalPanel.css';

export interface TerminalPanelProps {
  className?: string;
}

/** Detect the default shell command for the current OS. */
function getShellProgram(): string {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('win')) return 'powershell';
  return '/bin/sh';
}

/**
 * Resolve CSS variable value from the document root.
 * Falls back to the provided default when the variable is not set.
 */
function cssVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

export function TerminalPanel({ className }: TerminalPanelProps) {
  const { t } = useTranslation();
  const terminalVisible = useUiStore((s) => s.terminalVisible);
  const setTerminalVisible = useUiStore((s) => s.setTerminalVisible);
  const activeTabId = useRepoStore((s) => s.activeTabId);
  const tabs = useRepoStore((s) => s.tabs);

  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const childRef = useRef<Awaited<ReturnType<Command<string>['spawn']>> | null>(null);
  const spawnedPathRef = useRef<string | null>(null);

  // Derive the active repo path from the store
  const repoPath = activeTabId ? tabs.get(activeTabId)?.repoPath ?? null : null;

  /** Kill the running shell process (if any). */
  const killShell = useCallback(async () => {
    if (childRef.current) {
      try {
        await childRef.current.kill();
      } catch {
        // process may already be dead — ignore
      }
      childRef.current = null;
      spawnedPathRef.current = null;
    }
  }, []);

  /** Spawn a new shell process in the given working directory. */
  const spawnShell = useCallback(
    async (cwd: string) => {
      const term = termRef.current;
      if (!term) return;

      await killShell();

      try {
        const shellProgram = getShellProgram();
        const cmd = Command.create(shellProgram, [], {
          cwd,
          encoding: 'utf-8',
        });

        cmd.on('close', () => {
          term.writeln('\r\n[Process exited]');
          childRef.current = null;
          spawnedPathRef.current = null;
        });

        cmd.stdout.on('data', (data: string) => {
          term.write(data);
        });

        cmd.stderr.on('data', (data: string) => {
          term.write(data);
        });

        const child = await cmd.spawn();
        childRef.current = child;
        spawnedPathRef.current = cwd;

        // Forward user keystrokes to the shell process stdin
        term.onData((data: string) => {
          child.write(data + '\n');
        });
      } catch (err) {
        term.writeln(`\r\n[Failed to spawn shell: ${String(err)}]`);
      }
    },
    [killShell],
  );

  // --- Initialise xterm Terminal once on mount ---
  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: cssVar('--color-bg-tertiary', '#11111b'),
        foreground: cssVar('--color-text-primary', '#cdd6f4'),
        cursor: cssVar('--color-accent', '#89b4fa'),
      },
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);

    // Initial fit
    try {
      fitAddon.fit();
    } catch {
      // container may not be visible yet
    }

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    return () => {
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, []);

  // --- Fit terminal on resize & visibility changes ---
  useEffect(() => {
    const fitAddon = fitAddonRef.current;
    if (!fitAddon) return;

    const handleResize = () => {
      try {
        fitAddon.fit();
      } catch {
        // ignore if not visible
      }
    };

    // Fit when the panel becomes visible
    if (terminalVisible) {
      // Small delay to let the DOM settle
      const timer = setTimeout(handleResize, 50);
      window.addEventListener('resize', handleResize);
      return () => {
        clearTimeout(timer);
        window.removeEventListener('resize', handleResize);
      };
    }
  }, [terminalVisible]);

  // --- ResizeObserver for the container ---
  useEffect(() => {
    const el = containerRef.current;
    const fitAddon = fitAddonRef.current;
    if (!el || !fitAddon) return;

    const observer = new ResizeObserver(() => {
      try {
        fitAddon.fit();
      } catch {
        // ignore
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // --- Spawn / re-spawn shell when repoPath changes (tab switch) ---
  useEffect(() => {
    if (!repoPath) return;
    // Only re-spawn if the path actually changed
    if (spawnedPathRef.current === repoPath) return;

    const term = termRef.current;
    if (term) {
      term.clear();
    }
    spawnShell(repoPath);
  }, [repoPath, spawnShell]);

  // --- Cleanup shell on unmount ---
  useEffect(() => {
    return () => {
      killShell();
    };
  }, [killShell]);

  const handleClose = useCallback(() => {
    setTerminalVisible(false);
  }, [setTerminalVisible]);

  return (
    <div
      className={`terminal-panel${terminalVisible ? '' : ' terminal-panel--hidden'}${className ? ` ${className}` : ''}`}
      role="region"
      aria-label={t('terminal.title')}
    >
      <div className="terminal-panel__header">
        <span className="terminal-panel__title">{t('terminal.title')}</span>
        <button
          className="terminal-panel__close-btn"
          type="button"
          onClick={handleClose}
          aria-label={t('terminal.hide')}
          title={t('terminal.hide')}
        >
          ✕
        </button>
      </div>
      <div className="terminal-panel__body" ref={containerRef} />
    </div>
  );
}
