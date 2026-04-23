import { useState, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { BranchInfo, TagInfo, StashEntry, SubmoduleInfo, WorktreeInfo, PullRequest, CreatePrParams } from '../../ipc/types';
import { filterByName } from './sidebarUtils';
import { LocalBranchesSection } from './LocalBranchesSection';
import { RemotesSection } from './RemotesSection';
import { TagsSection } from './TagsSection';
import { StashesSection } from './StashesSection';
import { SubmoduleInteractions } from './SubmoduleInteractions';
import { SubmoduleUpdateBanner } from './SubmoduleUpdateBanner';
import { WorktreeSection } from './WorktreeSection';
import { HostIntegrationSection, type HostPlatform } from './HostIntegrationSection';

export interface SidebarProps {
  tabId: string;
  localBranches: BranchInfo[];
  remoteBranches: BranchInfo[];
  tags: TagInfo[];
  stashes: StashEntry[];
  submodules: SubmoduleInfo[];
  soloedBranches: Set<string>;
  hiddenBranches: Set<string>;
  // Branch callbacks
  onSolo: (branch: string) => void;
  onHide: (branch: string) => void;
  onCreateBranch: () => void;
  onDeleteBranch: (name: string) => void;
  onCheckout: (name: string) => void;
  // Remote callbacks
  onFetch: (remote: string) => void;
  // Tag callbacks
  onCheckoutTag: (name: string) => void;
  onDeleteTag: (name: string) => void;
  onPushTag: (name: string) => void;
  onCopyTagName: (name: string) => void;
  // Stash callbacks
  onApplyStash: (index: number) => void;
  onPopStash: (index: number) => void;
  onDropStash: (index: number) => void;
  onSelectStash: (index: number) => void;
  // Submodule callbacks
  onInitSubmodule: (path: string) => void;
  onUpdateSubmodule: (path: string) => void;
  onDeinitSubmodule: (path: string) => void;
  onOpenSubmodule: (path: string) => void;
  onCopySubmodulePath: (path: string) => void;
  onChangeSubmoduleUrl: (path: string) => void;
  // Submodule update banner
  showSubmoduleUpdateBanner?: boolean;
  changedSubmodules?: string[];
  onUpdateAllSubmodules?: () => void;
  onDismissSubmoduleBanner?: () => void;
  // Worktree callbacks
  worktrees?: WorktreeInfo[];
  onSelectWorktree?: (worktree: WorktreeInfo) => void;
  onDeleteWorktree?: (name: string) => void;
  // Host integration
  hostPlatform?: HostPlatform;
  onHostPlatformChange?: (platform: HostPlatform) => void;
  hostAuthenticated?: boolean;
  onHostAuthenticate?: (platform: HostPlatform, token: string) => Promise<void>;
  onHostLogout?: () => void;
  hostPullRequests?: PullRequest[];
  hostLoadingPrs?: boolean;
  onHostRefreshPrs?: () => void;
  onHostCreatePr?: (params: CreatePrParams) => Promise<void>;
  hostError?: string | null;
}

export function Sidebar(props: SidebarProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');

  const handleSearchChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => setSearchQuery(e.target.value),
    [],
  );

  // Filter all sections by the search query
  const filteredLocal = useMemo(
    () => filterByName(props.localBranches, searchQuery, (b) => b.name),
    [props.localBranches, searchQuery],
  );
  const filteredRemote = useMemo(
    () => filterByName(props.remoteBranches, searchQuery, (b) => b.name),
    [props.remoteBranches, searchQuery],
  );
  const filteredTags = useMemo(
    () => filterByName(props.tags, searchQuery, (t) => t.name),
    [props.tags, searchQuery],
  );
  const filteredStashes = useMemo(
    () =>
      filterByName(props.stashes, searchQuery, (s) => s.message || `stash@{${s.index}}`),
    [props.stashes, searchQuery],
  );
  const filteredSubmodules = useMemo(
    () => filterByName(props.submodules, searchQuery, (s) => s.name),
    [props.submodules, searchQuery],
  );
  const filteredWorktrees = useMemo(
    () => filterByName(props.worktrees ?? [], searchQuery, (w) => w.name),
    [props.worktrees, searchQuery],
  );

  return (
    <aside className="sidebar" role="complementary" aria-label="Repository sidebar">
      {/* Submodule update banner (shown after pull when refs changed) */}
      <SubmoduleUpdateBanner
        show={props.showSubmoduleUpdateBanner ?? false}
        submodules={props.changedSubmodules ?? []}
        onUpdateAll={props.onUpdateAllSubmodules ?? (() => {})}
        onDismiss={props.onDismissSubmoduleBanner ?? (() => {})}
      />

      {/* Search */}
      <div className="sidebar-search">
        <input
          type="search"
          className="sidebar-search-input"
          placeholder={t('sidebar.search')}
          value={searchQuery}
          onChange={handleSearchChange}
          aria-label={t('sidebar.search')}
        />
      </div>

      {/* Sections */}
      <div className="sidebar-sections">
        <LocalBranchesSection
          branches={filteredLocal}
          tabId={props.tabId}
          soloedBranches={props.soloedBranches}
          hiddenBranches={props.hiddenBranches}
          onSolo={props.onSolo}
          onHide={props.onHide}
          onCreateBranch={props.onCreateBranch}
          onDeleteBranch={props.onDeleteBranch}
          onCheckout={props.onCheckout}
        />

        <RemotesSection
          branches={filteredRemote}
          soloedBranches={props.soloedBranches}
          hiddenBranches={props.hiddenBranches}
          onSolo={props.onSolo}
          onHide={props.onHide}
          onFetch={props.onFetch}
        />

        <StashesSection
          stashes={filteredStashes}
          onApply={props.onApplyStash}
          onPop={props.onPopStash}
          onDrop={props.onDropStash}
          onSelect={props.onSelectStash}
        />

        <TagsSection
          tags={filteredTags}
          onCheckoutTag={props.onCheckoutTag}
          onDeleteTag={props.onDeleteTag}
          onPushTag={props.onPushTag}
          onCopyTagName={props.onCopyTagName}
        />

        <SubmoduleInteractions
          tabId={props.tabId}
          submodules={filteredSubmodules}
          onInit={props.onInitSubmodule}
          onUpdate={props.onUpdateSubmodule}
          onDeinit={props.onDeinitSubmodule}
          onOpen={props.onOpenSubmodule}
          onCopyPath={props.onCopySubmodulePath}
          onChangeUrl={props.onChangeSubmoduleUrl}
        />

        <WorktreeSection
          worktrees={filteredWorktrees}
          onSelect={props.onSelectWorktree ?? (() => {})}
          onDelete={props.onDeleteWorktree ?? (() => {})}
        />

        <HostIntegrationSection
          platform={props.hostPlatform ?? 'github'}
          onPlatformChange={props.onHostPlatformChange ?? (() => {})}
          isAuthenticated={props.hostAuthenticated ?? false}
          onAuthenticate={props.onHostAuthenticate ?? (async () => {})}
          onLogout={props.onHostLogout ?? (() => {})}
          pullRequests={props.hostPullRequests ?? []}
          loadingPrs={props.hostLoadingPrs ?? false}
          onRefreshPrs={props.onHostRefreshPrs ?? (() => {})}
          onCreatePr={props.onHostCreatePr ?? (async () => {})}
          error={props.hostError ?? null}
          localBranches={props.localBranches.map((b) => b.name)}
        />
      </div>
    </aside>
  );
}
