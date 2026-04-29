import { describe, it, expect } from 'vitest';
import { filterRepos } from '../filterRepos';
import type { RepoEntry } from '../../../ipc/types';

const makeRepo = (name: string, path: string): RepoEntry => ({
  name,
  path,
  last_opened: new Date().toISOString(),
});

const repos: RepoEntry[] = [
  makeRepo('my-project', '/home/user/my-project'),
  makeRepo('Work App', 'C:\\Users\\dev\\work-app'),
  makeRepo('open-source-lib', '/opt/repos/open-source-lib'),
];

describe('filterRepos', () => {
  it('returns the full list when query is empty', () => {
    expect(filterRepos(repos, '')).toEqual(repos);
  });

  it('returns the full list when query is whitespace-only', () => {
    expect(filterRepos(repos, '   ')).toEqual(repos);
    expect(filterRepos(repos, '\t\n')).toEqual(repos);
  });

  it('filters by case-insensitive name match', () => {
    const result = filterRepos(repos, 'work');
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('Work App');
  });

  it('filters by case-insensitive path match', () => {
    const result = filterRepos(repos, '/opt/repos');
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('open-source-lib');
  });

  it('matches across both name and path', () => {
    // "my-project" appears in both name and path of the first entry
    const result = filterRepos(repos, 'my-project');
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('my-project');
  });

  it('returns empty array when nothing matches', () => {
    expect(filterRepos(repos, 'nonexistent')).toEqual([]);
  });

  it('handles empty repos list', () => {
    expect(filterRepos([], 'query')).toEqual([]);
    expect(filterRepos([], '')).toEqual([]);
  });
});
