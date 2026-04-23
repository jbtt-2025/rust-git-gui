import { create } from 'zustand';
import type {
  RepoEntry,
  RepositoryState,
} from '../ipc/types';

export interface TabState {
  tabId: string;
  repoName: string;
  repoPath: string;
  hasChanges: boolean;
  repoState: RepositoryState;
  soloedBranches: Set<string>;
  hiddenBranches: Set<string>;
  pinnedLeftBranches: string[];
}

interface RepoStoreState {
  activeTabId: string | null;
  tabs: Map<string, TabState>;
  recentRepos: RepoEntry[];
}

interface RepoStoreActions {
  setActiveTab: (tabId: string) => void;
  addTab: (tab: TabState) => void;
  removeTab: (tabId: string) => void;
  updateTab: (tabId: string, partial: Partial<TabState>) => void;
  setRecentRepos: (repos: RepoEntry[]) => void;

  // Tab reordering
  reorderTabs: (fromTabId: string, toTabId: string) => void;

  // Solo / Hide / Pin to Left
  toggleSolo: (tabId: string, branch: string) => void;
  toggleHide: (tabId: string, branch: string) => void;
  togglePinLeft: (tabId: string, branch: string) => void;
  resetView: (tabId: string) => void;
}

export const useRepoStore = create<RepoStoreState & RepoStoreActions>(
  (set) => ({
    activeTabId: null,
    tabs: new Map(),
    recentRepos: [],

    setActiveTab: (tabId) => set({ activeTabId: tabId }),

    addTab: (tab) =>
      set((state) => {
        const tabs = new Map(state.tabs);
        tabs.set(tab.tabId, tab);
        return { tabs, activeTabId: tab.tabId };
      }),

    removeTab: (tabId) =>
      set((state) => {
        const tabs = new Map(state.tabs);
        tabs.delete(tabId);
        const activeTabId =
          state.activeTabId === tabId
            ? (tabs.keys().next().value ?? null)
            : state.activeTabId;
        return { tabs, activeTabId };
      }),

    updateTab: (tabId, partial) =>
      set((state) => {
        const existing = state.tabs.get(tabId);
        if (!existing) return state;
        const tabs = new Map(state.tabs);
        tabs.set(tabId, { ...existing, ...partial });
        return { tabs };
      }),

    setRecentRepos: (repos) => set({ recentRepos: repos }),

    reorderTabs: (fromTabId, toTabId) =>
      set((state) => {
        if (fromTabId === toTabId) return state;
        const entries = Array.from(state.tabs.entries());
        const fromIdx = entries.findIndex(([id]) => id === fromTabId);
        const toIdx = entries.findIndex(([id]) => id === toTabId);
        if (fromIdx === -1 || toIdx === -1) return state;
        const [moved] = entries.splice(fromIdx, 1);
        entries.splice(toIdx, 0, moved);
        return { tabs: new Map(entries) };
      }),

    toggleSolo: (tabId, branch) =>
      set((state) => {
        const tab = state.tabs.get(tabId);
        if (!tab) return state;
        const soloed = new Set(tab.soloedBranches);
        if (soloed.has(branch)) {
          soloed.delete(branch);
        } else {
          soloed.add(branch);
        }
        const tabs = new Map(state.tabs);
        tabs.set(tabId, { ...tab, soloedBranches: soloed });
        return { tabs };
      }),

    toggleHide: (tabId, branch) =>
      set((state) => {
        const tab = state.tabs.get(tabId);
        if (!tab) return state;
        const hidden = new Set(tab.hiddenBranches);
        if (hidden.has(branch)) {
          hidden.delete(branch);
        } else {
          hidden.add(branch);
        }
        const tabs = new Map(state.tabs);
        tabs.set(tabId, { ...tab, hiddenBranches: hidden });
        return { tabs };
      }),

    togglePinLeft: (tabId, branch) =>
      set((state) => {
        const tab = state.tabs.get(tabId);
        if (!tab) return state;
        const pinned = [...tab.pinnedLeftBranches];
        const idx = pinned.indexOf(branch);
        if (idx >= 0) {
          pinned.splice(idx, 1);
        } else {
          pinned.push(branch);
        }
        const tabs = new Map(state.tabs);
        tabs.set(tabId, { ...tab, pinnedLeftBranches: pinned });
        return { tabs };
      }),

    resetView: (tabId) =>
      set((state) => {
        const tab = state.tabs.get(tabId);
        if (!tab) return state;
        const tabs = new Map(state.tabs);
        tabs.set(tabId, {
          ...tab,
          soloedBranches: new Set(),
          hiddenBranches: new Set(),
          pinnedLeftBranches: [],
        });
        return { tabs };
      }),
  }),
);
