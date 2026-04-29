import type { RepoEntry } from '../../ipc/types';

/**
 * Filter the recent repositories list.
 * Performs case-insensitive substring matching on both name and path.
 */
export function filterRepos(repos: RepoEntry[], query: string): RepoEntry[] {
  if (!query.trim()) return repos;
  const q = query.toLowerCase();
  return repos.filter(
    (r) => r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q)
  );
}
