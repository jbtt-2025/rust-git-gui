import { create } from 'zustand';
import type { ThemeMode } from '../ipc/types';

interface UiStoreState {
  theme: ThemeMode;
  sidebarCollapsed: Record<string, boolean>;
  terminalVisible: boolean;
  fullscreen: boolean;
  diffViewMode: 'split' | 'inline';
}

interface UiStoreActions {
  setTheme: (theme: ThemeMode) => void;
  toggleSidebarSection: (section: string) => void;
  setSidebarCollapsed: (section: string, collapsed: boolean) => void;
  setTerminalVisible: (visible: boolean) => void;
  toggleTerminal: () => void;
  setFullscreen: (fullscreen: boolean) => void;
  toggleFullscreen: () => void;
  setDiffViewMode: (mode: 'split' | 'inline') => void;
}

export const useUiStore = create<UiStoreState & UiStoreActions>((set) => ({
  theme: 'System',
  sidebarCollapsed: {},
  terminalVisible: false,
  fullscreen: false,
  diffViewMode: 'split',

  setTheme: (theme) => set({ theme }),

  toggleSidebarSection: (section) =>
    set((state) => ({
      sidebarCollapsed: {
        ...state.sidebarCollapsed,
        [section]: !state.sidebarCollapsed[section],
      },
    })),

  setSidebarCollapsed: (section, collapsed) =>
    set((state) => ({
      sidebarCollapsed: {
        ...state.sidebarCollapsed,
        [section]: collapsed,
      },
    })),

  setTerminalVisible: (visible) => set({ terminalVisible: visible }),
  toggleTerminal: () =>
    set((state) => ({ terminalVisible: !state.terminalVisible })),

  setFullscreen: (fullscreen) => set({ fullscreen }),
  toggleFullscreen: () =>
    set((state) => ({ fullscreen: !state.fullscreen })),

  setDiffViewMode: (mode) => set({ diffViewMode: mode }),
}));
