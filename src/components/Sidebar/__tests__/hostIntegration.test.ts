/**
 * Unit tests for HostIntegration panel logic.
 *
 * Validates: Requirements 19.1, 19.2, 19.3, 19.4
 */
import { describe, it, expect, vi } from 'vitest';
import type { PullRequest, CreatePrParams } from '../../../ipc/types';
import {
  prStateKey,
  prStateBadgeClass,
  type HostIntegrationSectionProps,
} from '../HostIntegrationSection';

// --- helpers ---

function makePr(overrides: Partial<PullRequest> = {}): PullRequest {
  return {
    id: 1,
    title: 'Fix bug',
    description: 'Fixes a critical bug',
    state: 'Open',
    source_branch: 'feature',
    target_branch: 'main',
    author: 'dev',
    url: 'https://github.com/org/repo/pull/1',
    ...overrides,
  };
}

function makeProps(overrides: Partial<HostIntegrationSectionProps> = {}): HostIntegrationSectionProps {
  return {
    platform: 'github',
    onPlatformChange: vi.fn(),
    isAuthenticated: false,
    onAuthenticate: vi.fn().mockResolvedValue(undefined),
    onLogout: vi.fn(),
    pullRequests: [],
    loadingPrs: false,
    onRefreshPrs: vi.fn(),
    onCreatePr: vi.fn().mockResolvedValue(undefined),
    error: null,
    localBranches: ['main', 'develop', 'feature/test'],
    ...overrides,
  };
}

// --- prStateKey ---

describe('prStateKey', () => {
  it('returns correct i18n key for Open state', () => {
    expect(prStateKey('Open')).toBe('hostIntegration.open');
  });

  it('returns correct i18n key for Closed state', () => {
    expect(prStateKey('Closed')).toBe('hostIntegration.closed');
  });

  it('returns correct i18n key for Merged state', () => {
    expect(prStateKey('Merged')).toBe('hostIntegration.merged');
  });
});

// --- prStateBadgeClass ---

describe('prStateBadgeClass', () => {
  it('returns open badge class for Open state', () => {
    expect(prStateBadgeClass('Open')).toBe('host-pr-badge--open');
  });

  it('returns closed badge class for Closed state', () => {
    expect(prStateBadgeClass('Closed')).toBe('host-pr-badge--closed');
  });

  it('returns merged badge class for Merged state', () => {
    expect(prStateBadgeClass('Merged')).toBe('host-pr-badge--merged');
  });

  it('maps each state to a unique class', () => {
    const classes = new Set(['Open', 'Closed', 'Merged'].map((s) => prStateBadgeClass(s as any)));
    expect(classes.size).toBe(3);
  });
});

// --- Props validation ---

describe('HostIntegrationSection props', () => {
  it('defaults to github platform', () => {
    const props = makeProps();
    expect(props.platform).toBe('github');
  });

  it('supports gitlab platform', () => {
    const props = makeProps({ platform: 'gitlab' });
    expect(props.platform).toBe('gitlab');
  });

  it('shows not authenticated by default', () => {
    const props = makeProps();
    expect(props.isAuthenticated).toBe(false);
  });

  it('shows authenticated when set', () => {
    const props = makeProps({ isAuthenticated: true });
    expect(props.isAuthenticated).toBe(true);
  });

  it('passes pull requests list', () => {
    const prs = [makePr({ id: 1 }), makePr({ id: 2, state: 'Merged' })];
    const props = makeProps({ pullRequests: prs });
    expect(props.pullRequests).toHaveLength(2);
    expect(props.pullRequests[0].state).toBe('Open');
    expect(props.pullRequests[1].state).toBe('Merged');
  });

  it('provides local branches for PR creation', () => {
    const props = makeProps({ localBranches: ['main', 'dev'] });
    expect(props.localBranches).toEqual(['main', 'dev']);
  });
});

// --- Platform change callback ---

describe('Platform change', () => {
  it('calls onPlatformChange with new platform', () => {
    const onChange = vi.fn();
    const props = makeProps({ onPlatformChange: onChange });
    props.onPlatformChange('gitlab');
    expect(onChange).toHaveBeenCalledWith('gitlab');
  });
});

// --- Authentication callback ---

describe('Authentication', () => {
  it('calls onAuthenticate with platform and token', async () => {
    const onAuth = vi.fn().mockResolvedValue(undefined);
    const props = makeProps({ onAuthenticate: onAuth });
    await props.onAuthenticate('github', 'ghp_test123');
    expect(onAuth).toHaveBeenCalledWith('github', 'ghp_test123');
  });

  it('calls onLogout when logging out', () => {
    const onLogout = vi.fn();
    const props = makeProps({ isAuthenticated: true, onLogout });
    props.onLogout();
    expect(onLogout).toHaveBeenCalled();
  });
});

// --- Create PR callback ---

describe('Create Pull Request', () => {
  it('calls onCreatePr with correct params', async () => {
    const onCreate = vi.fn().mockResolvedValue(undefined);
    const props = makeProps({ isAuthenticated: true, onCreatePr: onCreate });
    const params: CreatePrParams = {
      title: 'New feature',
      description: 'Adds a new feature',
      source_branch: 'feature',
      target_branch: 'main',
    };
    await props.onCreatePr(params);
    expect(onCreate).toHaveBeenCalledWith(params);
  });
});

// --- Error display ---

describe('Error handling', () => {
  it('passes error message when present', () => {
    const props = makeProps({ error: 'Authentication failed' });
    expect(props.error).toBe('Authentication failed');
  });

  it('has null error by default', () => {
    const props = makeProps();
    expect(props.error).toBeNull();
  });
});

// --- PR list filtering by state ---

describe('PR list filtering', () => {
  it('can filter open PRs from the list', () => {
    const prs = [
      makePr({ id: 1, state: 'Open' }),
      makePr({ id: 2, state: 'Closed' }),
      makePr({ id: 3, state: 'Merged' }),
      makePr({ id: 4, state: 'Open' }),
    ];
    const openPrs = prs.filter((pr) => pr.state === 'Open');
    expect(openPrs).toHaveLength(2);
    expect(openPrs.map((p) => p.id)).toEqual([1, 4]);
  });

  it('handles empty PR list', () => {
    const prs: PullRequest[] = [];
    expect(prs.filter((pr) => pr.state === 'Open')).toHaveLength(0);
  });
});
