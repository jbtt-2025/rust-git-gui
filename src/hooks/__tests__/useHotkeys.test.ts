import { describe, it, expect } from 'vitest';
import { parseHotkeyBinding, formatKeyEvent, findHotkeyConflict } from '../useHotkeys';

describe('parseHotkeyBinding', () => {
  it('parses simple key binding', () => {
    expect(parseHotkeyBinding('Ctrl+Enter')).toEqual({
      ctrl: true, shift: false, alt: false, key: 'Enter',
    });
  });

  it('parses multi-modifier binding', () => {
    expect(parseHotkeyBinding('Ctrl+Shift+S')).toEqual({
      ctrl: true, shift: true, alt: false, key: 'S',
    });
  });

  it('parses Alt modifier', () => {
    expect(parseHotkeyBinding('Alt+F4')).toEqual({
      ctrl: false, shift: false, alt: true, key: 'F4',
    });
  });

  it('parses Meta as ctrl', () => {
    expect(parseHotkeyBinding('Meta+K')).toEqual({
      ctrl: true, shift: false, alt: false, key: 'K',
    });
  });

  it('parses backtick binding', () => {
    expect(parseHotkeyBinding('Ctrl+`')).toEqual({
      ctrl: true, shift: false, alt: false, key: '`',
    });
  });

  it('parses all modifiers', () => {
    expect(parseHotkeyBinding('Ctrl+Shift+Alt+X')).toEqual({
      ctrl: true, shift: true, alt: true, key: 'X',
    });
  });
});

describe('formatKeyEvent', () => {
  function makeEvent(overrides: Partial<KeyboardEvent>): KeyboardEvent {
    return {
      key: 'a',
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      metaKey: false,
      ...overrides,
    } as KeyboardEvent;
  }

  it('returns null for bare modifier keys', () => {
    expect(formatKeyEvent(makeEvent({ key: 'Control' }))).toBeNull();
    expect(formatKeyEvent(makeEvent({ key: 'Shift' }))).toBeNull();
    expect(formatKeyEvent(makeEvent({ key: 'Alt' }))).toBeNull();
    expect(formatKeyEvent(makeEvent({ key: 'Meta' }))).toBeNull();
  });

  it('formats Ctrl+letter', () => {
    expect(formatKeyEvent(makeEvent({ key: 's', ctrlKey: true }))).toBe('Ctrl+S');
  });

  it('formats Ctrl+Shift+letter', () => {
    expect(
      formatKeyEvent(makeEvent({ key: 'z', ctrlKey: true, shiftKey: true })),
    ).toBe('Ctrl+Shift+Z');
  });

  it('formats space key', () => {
    expect(formatKeyEvent(makeEvent({ key: ' ', ctrlKey: true }))).toBe('Ctrl+Space');
  });

  it('formats Enter key', () => {
    expect(formatKeyEvent(makeEvent({ key: 'Enter', ctrlKey: true }))).toBe('Ctrl+Enter');
  });

  it('formats metaKey as Ctrl', () => {
    expect(formatKeyEvent(makeEvent({ key: 'm', metaKey: true }))).toBe('Ctrl+M');
  });
});

describe('findHotkeyConflict', () => {
  const hotkeys: Record<string, string> = {
    commit: 'Ctrl+Enter',
    stageAll: 'Ctrl+Shift+S',
    search: 'Ctrl+F',
  };

  it('returns null when no conflict', () => {
    expect(findHotkeyConflict(hotkeys, 'commit', 'Ctrl+K')).toBeNull();
  });

  it('returns conflicting action name', () => {
    expect(findHotkeyConflict(hotkeys, 'commit', 'Ctrl+F')).toBe('search');
  });

  it('does not conflict with itself', () => {
    expect(findHotkeyConflict(hotkeys, 'commit', 'Ctrl+Enter')).toBeNull();
  });

  it('is case-insensitive for key comparison', () => {
    expect(findHotkeyConflict(hotkeys, 'commit', 'Ctrl+f')).toBe('search');
  });
});
