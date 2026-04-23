import type { ThemeMode } from '../ipc/types';
import './dark.css';
import './light.css';

/** Resolve the effective theme ('Light' | 'Dark') from a ThemeMode. */
function resolveTheme(mode: ThemeMode): 'light' | 'dark' {
  if (mode === 'Light') return 'light';
  if (mode === 'Dark') return 'dark';
  // System — follow OS preference
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

/** Apply a theme to the document root. Takes effect immediately. */
export function applyTheme(mode: ThemeMode): void {
  const resolved = resolveTheme(mode);
  document.documentElement.setAttribute('data-theme', resolved);
}

/** Apply custom font settings to the document root. */
export function applyFont(family: string, size: number): void {
  document.documentElement.style.setProperty('--font-code-family', family);
  document.documentElement.style.setProperty(
    '--font-code-size',
    `${size}px`,
  );
}

let systemThemeCleanup: (() => void) | null = null;

/**
 * Start listening for OS theme changes. When mode is 'System', the theme
 * will automatically switch when the user toggles their OS dark/light mode.
 * Call this once on app startup and whenever the theme mode changes.
 */
export function watchSystemTheme(mode: ThemeMode): void {
  // Clean up previous listener
  if (systemThemeCleanup) {
    systemThemeCleanup();
    systemThemeCleanup = null;
  }

  if (mode !== 'System') return;

  const mql = window.matchMedia('(prefers-color-scheme: dark)');
  const handler = () => applyTheme('System');
  mql.addEventListener('change', handler);
  systemThemeCleanup = () => mql.removeEventListener('change', handler);
}
