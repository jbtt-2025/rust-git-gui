import type { CommitInfo, RefLabel } from '../../ipc/types';

/**
 * Node type classification for context menu item availability.
 * - 'local-branch': commit has at least one local branch ref
 * - 'remote-branch': commit has remote branch refs but no local branch
 * - 'bare-commit': commit has no branch refs (may have tags)
 */
export type NodeType = 'local-branch' | 'remote-branch' | 'bare-commit';

export interface MenuItem {
  id: string;
  labelKey: string;
  enabled: boolean;
  danger?: boolean;
  separator?: boolean;
  children?: MenuItem[];
}

/** Classify a commit node based on its refs. */
export function classifyNode(commit: CommitInfo): NodeType {
  const hasLocal = commit.refs.some((r) => r.ref_type.type === 'LocalBranch');
  const hasRemote = commit.refs.some((r) => r.ref_type.type === 'RemoteBranch');

  if (hasLocal) return 'local-branch';
  if (hasRemote) return 'remote-branch';
  return 'bare-commit';
}

/** Get the primary local branch name from refs, if any. */
export function getLocalBranchName(refs: RefLabel[]): string | null {
  const local = refs.find((r) => r.ref_type.type === 'LocalBranch');
  return local?.name ?? null;
}

/** Get the primary remote branch name from refs, if any. */
export function getRemoteBranchName(refs: RefLabel[]): string | null {
  const remote = refs.find((r) => r.ref_type.type === 'RemoteBranch');
  return remote?.name ?? null;
}

/**
 * Build the full context menu items list for a commit node.
 * Items are enabled/disabled based on the node type.
 */
export function getMenuItemsForNode(nodeType: NodeType): MenuItem[] {
  const isLocal = nodeType === 'local-branch';
  const isRemote = nodeType === 'remote-branch';
  const hasBranch = isLocal || isRemote;

  return [
    // Remote operations
    { id: 'pull', labelKey: 'remote.pull', enabled: isLocal },
    { id: 'push', labelKey: 'remote.push', enabled: isLocal },
    { id: 'set-upstream', labelKey: 'branch.setUpstream', enabled: isLocal },
    { id: 'separator-1', labelKey: '', enabled: true, separator: true },

    // Branch operations
    { id: 'merge', labelKey: 'branch.merge', enabled: hasBranch },
    { id: 'rebase', labelKey: 'rebase.start', enabled: hasBranch },
    { id: 'checkout', labelKey: 'branch.checkout', enabled: true },
    { id: 'create-worktree', labelKey: 'worktree.create', enabled: hasBranch },
    { id: 'create-branch', labelKey: 'branch.create', enabled: true },
    { id: 'separator-2', labelKey: '', enabled: true, separator: true },

    // Commit operations
    { id: 'cherry-pick', labelKey: 'cherryPick.title', enabled: true },
    {
      id: 'reset',
      labelKey: 'reset.soft',
      enabled: true,
      children: [
        { id: 'reset-soft', labelKey: 'reset.soft', enabled: true },
        { id: 'reset-mixed', labelKey: 'reset.mixed', enabled: true },
        { id: 'reset-hard', labelKey: 'reset.hard', enabled: true, danger: true },
      ],
    },
    { id: 'revert', labelKey: 'revert.title', enabled: true },
    { id: 'separator-3', labelKey: '', enabled: true, separator: true },

    // Branch management (only for nodes with branches)
    { id: 'rename-branch', labelKey: 'branch.rename', enabled: isLocal },
    { id: 'delete-branch', labelKey: 'branch.delete', enabled: hasBranch, danger: true },
    { id: 'separator-4', labelKey: '', enabled: true, separator: true },

    // Copy operations
    { id: 'copy-branch-name', labelKey: 'contextMenu.copyBranchName', enabled: hasBranch },
    { id: 'copy-commit-sha', labelKey: 'contextMenu.copyCommitSha', enabled: true },
    { id: 'copy-link-branch', labelKey: 'contextMenu.copyLinkToBranch', enabled: hasBranch },
    { id: 'copy-link-commit', labelKey: 'contextMenu.copyLinkToCommit', enabled: true },
    { id: 'create-patch', labelKey: 'contextMenu.createPatch', enabled: true },
    { id: 'separator-5', labelKey: '', enabled: true, separator: true },

    // View operations
    { id: 'hide', labelKey: 'contextMenu.hide', enabled: hasBranch },
    { id: 'pin-to-left', labelKey: 'contextMenu.pinToLeft', enabled: hasBranch },
    { id: 'solo', labelKey: 'contextMenu.solo', enabled: hasBranch },
    { id: 'separator-6', labelKey: '', enabled: true, separator: true },

    // Tag operations
    { id: 'create-tag', labelKey: 'tag.create', enabled: true },
    { id: 'create-annotated-tag', labelKey: 'tag.createAnnotated', enabled: true },
  ];
}
