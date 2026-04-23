import type { BranchInfo, TagInfo, StashEntry, SubmoduleInfo, RemoteInfo } from '../../ipc/types';

/** Filter items by a case-insensitive search query against their name. */
export function filterByName<T>(items: T[], query: string, getName: (item: T) => string): T[] {
  if (!query.trim()) return items;
  const lower = query.toLowerCase();
  return items.filter((item) => getName(item).toLowerCase().includes(lower));
}

/** Group remote branches by remote name. */
export function groupBranchesByRemote(branches: BranchInfo[]): Map<string, BranchInfo[]> {
  const map = new Map<string, BranchInfo[]>();
  for (const b of branches) {
    if (b.branch_type.type === 'Remote') {
      const remote = b.branch_type.remote_name;
      const list = map.get(remote) ?? [];
      list.push(b);
      map.set(remote, list);
    }
  }
  return map;
}

/** Strip the remote prefix from a remote branch name (e.g. "origin/main" → "main"). */
export function stripRemotePrefix(name: string): string {
  const idx = name.indexOf('/');
  return idx >= 0 ? name.slice(idx + 1) : name;
}

export type SidebarData = {
  localBranches: BranchInfo[];
  remoteBranches: BranchInfo[];
  tags: TagInfo[];
  stashes: StashEntry[];
  submodules: SubmoduleInfo[];
  remotes: RemoteInfo[];
};
