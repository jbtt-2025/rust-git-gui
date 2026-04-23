import { useEffect, useCallback, useRef } from 'react';
import { useSettingsStore } from '../stores/settingsStore';

export interface ParsedBinding {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  key: string;
}

const isMac =
  typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform);

/**
 * Parse a hotkey binding string like "Ctrl+Shift+S" into its components.
 */
export function parseHotkeyBinding(binding: string): ParsedBinding {
  const parts = binding.split('+');
  const result: ParsedBinding = { ctrl: false, shift: false, alt: false, key: '' };

  for (const part of parts) {
    const lower = part.toLowerCase();
    if (lower === 'ctrl' || lower === 'meta') {
      result.ctrl = true;
    } else if (lower === 'shift') {
      result.shift = true;
    } else if (lower === 'alt') {
      result.alt = true;
    } else {
      result.key = part;
    }
  }

  return result;
}

/**
 * Format a KeyboardEvent into a binding string (e.g. "Ctrl+Shift+S").
 * Useful for hotkey recording in Settings.
 */
export function formatKeyEvent(e: KeyboardEvent): string | null {
  // Ignore bare modifier keys
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');

  let key = e.key;
  if (key === ' ') key = 'Space';
  else if (key.length === 1) key = key.toUpperCase();
  else if (key === 'Backquote' || key === '`') key = '`';

  parts.push(key);
  return parts.join('+');
}

/**
 * Check if a new binding conflicts with an existing hotkey.
 * Returns the conflicting action name, or null if no conflict.
 */
export function findHotkeyConflict(
  hotkeys: Record<string, string>,
  action: string,
  newBinding: string,
): string | null {
  const normalizedNew = normalizeBinding(newBinding);
  for (const [existingAction, existingBinding] of Object.entries(hotkeys)) {
    if (existingAction === action) continue;
    if (normalizeBinding(existingBinding) === normalizedNew) {
      return existingAction;
    }
  }
  return null;
}

/** Normalize a binding string for comparison (lowercase key, sorted modifiers). */
function normalizeBinding(binding: string): string {
  const parsed = parseHotkeyBinding(binding);
  const parts: string[] = [];
  if (parsed.ctrl) parts.push('ctrl');
  if (parsed.shift) parts.push('shift');
  if (parsed.alt) parts.push('alt');
  parts.push(parsed.key.toLowerCase());
  return parts.join('+');
}

function matchesEvent(parsed: ParsedBinding, e: KeyboardEvent): boolean {
  // On macOS, "Ctrl" in bindings maps to metaKey (Cmd); on other platforms to ctrlKey
  const ctrlMatch = isMac ? e.metaKey : e.ctrlKey;
  if (parsed.ctrl !== ctrlMatch) return false;
  if (parsed.shift !== e.shiftKey) return false;
  if (parsed.alt !== e.altKey) return false;

  let eventKey = e.key;
  if (eventKey === ' ') eventKey = 'Space';
  else if (eventKey.length === 1) eventKey = eventKey.toUpperCase();
  else if (eventKey === 'Backquote') eventKey = '`';

  return eventKey.toLowerCase() === parsed.key.toLowerCase();
}

/**
 * Custom hook that registers global keyboard shortcuts.
 *
 * Reads hotkey bindings from settingsStore and maps them to handler callbacks.
 * The listener is re-registered whenever bindings change (immediate effect).
 *
 * @param handlers - Map of action names to callback functions
 */
export function useHotkeys(handlers: Record<string, () => void>): void {
  const hotkeys = useSettingsStore((s) => s.settings.hotkeys);
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  const onKeyDown = useCallback(
    (e: KeyboardEvent) => {
      for (const [action, binding] of Object.entries(hotkeys)) {
        const parsed = parseHotkeyBinding(binding);
        if (matchesEvent(parsed, e)) {
          const handler = handlersRef.current[action];
          if (handler) {
            e.preventDefault();
            handler();
          }
          return;
        }
      }
    },
    [hotkeys],
  );

  useEffect(() => {
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onKeyDown]);
}
