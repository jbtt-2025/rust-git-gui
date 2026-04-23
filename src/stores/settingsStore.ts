import { create } from 'zustand';
import type { AppSettings, CommitTemplate } from '../ipc/types';

const DEFAULT_SETTINGS: AppSettings = {
  theme: 'System',
  language: 'en',
  font_family: 'monospace',
  font_size: 14,
  hotkeys: {
    commit: 'Ctrl+Enter',
    stageAll: 'Ctrl+Shift+S',
    switchBranch: 'Ctrl+B',
    search: 'Ctrl+F',
    undo: 'Ctrl+Z',
    redo: 'Ctrl+Shift+Z',
    toggleTerminal: 'Ctrl+`',
    toggleFullscreen: 'Ctrl+M',
  },
  window: {
    width: 1280,
    height: 800,
    x: null,
    y: null,
    maximized: false,
  },
  commit_templates: [],
};

interface SettingsStoreState {
  settings: AppSettings;
  dirty: boolean;
}

interface SettingsStoreActions {
  loadSettings: (settings: AppSettings) => void;
  updateSettings: (partial: Partial<AppSettings>) => void;
  setLanguage: (language: string) => void;
  setFontFamily: (fontFamily: string) => void;
  setFontSize: (fontSize: number) => void;
  setHotkey: (action: string, binding: string) => void;
  addCommitTemplate: (template: CommitTemplate) => void;
  removeCommitTemplate: (id: string) => void;
  updateCommitTemplate: (id: string, partial: Partial<CommitTemplate>) => void;
  markClean: () => void;
}

export const useSettingsStore = create<
  SettingsStoreState & SettingsStoreActions
>((set) => ({
  settings: DEFAULT_SETTINGS,
  dirty: false,

  loadSettings: (settings) => set({ settings, dirty: false }),

  updateSettings: (partial) =>
    set((state) => ({
      settings: { ...state.settings, ...partial },
      dirty: true,
    })),

  setLanguage: (language) =>
    set((state) => ({
      settings: { ...state.settings, language },
      dirty: true,
    })),

  setFontFamily: (fontFamily) =>
    set((state) => ({
      settings: { ...state.settings, font_family: fontFamily },
      dirty: true,
    })),

  setFontSize: (fontSize) =>
    set((state) => ({
      settings: { ...state.settings, font_size: fontSize },
      dirty: true,
    })),

  setHotkey: (action, binding) =>
    set((state) => ({
      settings: {
        ...state.settings,
        hotkeys: { ...state.settings.hotkeys, [action]: binding },
      },
      dirty: true,
    })),

  addCommitTemplate: (template) =>
    set((state) => ({
      settings: {
        ...state.settings,
        commit_templates: [...state.settings.commit_templates, template],
      },
      dirty: true,
    })),

  removeCommitTemplate: (id) =>
    set((state) => ({
      settings: {
        ...state.settings,
        commit_templates: state.settings.commit_templates.filter(
          (t) => t.id !== id,
        ),
      },
      dirty: true,
    })),

  updateCommitTemplate: (id, partial) =>
    set((state) => ({
      settings: {
        ...state.settings,
        commit_templates: state.settings.commit_templates.map((t) =>
          t.id === id ? { ...t, ...partial } : t,
        ),
      },
      dirty: true,
    })),

  markClean: () => set({ dirty: false }),
}));
