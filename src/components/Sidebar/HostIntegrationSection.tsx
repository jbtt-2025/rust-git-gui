import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { PullRequest, CreatePrParams, PrState } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';

export type HostPlatform = 'github' | 'gitlab';

export interface HostIntegrationSectionProps {
  /** Currently selected platform */
  platform: HostPlatform;
  onPlatformChange: (platform: HostPlatform) => void;
  /** Whether the user is authenticated */
  isAuthenticated: boolean;
  /** Authenticate with a PAT */
  onAuthenticate: (platform: HostPlatform, token: string) => Promise<void>;
  /** Logout / clear auth */
  onLogout: () => void;
  /** Pull requests for the current repo */
  pullRequests: PullRequest[];
  /** Whether PRs are currently loading */
  loadingPrs: boolean;
  /** Refresh the PR list */
  onRefreshPrs: () => void;
  /** Create a new pull request */
  onCreatePr: (params: CreatePrParams) => Promise<void>;
  /** Error message to display (auth or PR errors) */
  error: string | null;
  /** Available local branches for source/target selection */
  localBranches: string[];
}

/** Map PrState to i18n key */
export function prStateKey(state: PrState): string {
  switch (state) {
    case 'Open': return 'hostIntegration.open';
    case 'Closed': return 'hostIntegration.closed';
    case 'Merged': return 'hostIntegration.merged';
  }
}

/** CSS class for PR state badge */
export function prStateBadgeClass(state: PrState): string {
  switch (state) {
    case 'Open': return 'host-pr-badge--open';
    case 'Closed': return 'host-pr-badge--closed';
    case 'Merged': return 'host-pr-badge--merged';
  }
}

export function HostIntegrationSection({
  platform,
  onPlatformChange,
  isAuthenticated,
  onAuthenticate,
  onLogout,
  pullRequests,
  loadingPrs,
  onRefreshPrs,
  onCreatePr,
  error,
  localBranches,
}: HostIntegrationSectionProps) {
  const { t } = useTranslation();

  // Auth form state
  const [token, setToken] = useState('');
  const [authenticating, setAuthenticating] = useState(false);

  // Create PR form state
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [prTitle, setPrTitle] = useState('');
  const [prDescription, setPrDescription] = useState('');
  const [sourceBranch, setSourceBranch] = useState('');
  const [targetBranch, setTargetBranch] = useState('');
  const [creating, setCreating] = useState(false);

  const handleAuthenticate = useCallback(async () => {
    if (!token.trim()) return;
    setAuthenticating(true);
    try {
      await onAuthenticate(platform, token.trim());
      setToken('');
    } finally {
      setAuthenticating(false);
    }
  }, [platform, token, onAuthenticate]);

  const handleCreatePr = useCallback(async () => {
    if (!prTitle.trim() || !sourceBranch || !targetBranch) return;
    setCreating(true);
    try {
      await onCreatePr({
        title: prTitle.trim(),
        description: prDescription.trim(),
        source_branch: sourceBranch,
        target_branch: targetBranch,
      });
      setPrTitle('');
      setPrDescription('');
      setSourceBranch('');
      setTargetBranch('');
      setShowCreateForm(false);
    } finally {
      setCreating(false);
    }
  }, [prTitle, prDescription, sourceBranch, targetBranch, onCreatePr]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') handleAuthenticate();
    },
    [handleAuthenticate],
  );

  return (
    <SidebarSection
      id="host-integration"
      title={t('hostIntegration.title')}
      count={pullRequests.length}
    >
      <div className="host-integration" data-testid="host-integration">
        {/* Platform selector */}
        <div className="host-platform-selector">
          <label className="host-label" htmlFor="host-platform-select">
            {t('hostIntegration.platform')}
          </label>
          <select
            id="host-platform-select"
            className="host-select"
            value={platform}
            onChange={(e) => onPlatformChange(e.target.value as HostPlatform)}
          >
            <option value="github">{t('hostIntegration.github')}</option>
            <option value="gitlab">{t('hostIntegration.gitlab')}</option>
          </select>
        </div>

        {/* Auth section */}
        {!isAuthenticated ? (
          <div className="host-auth">
            <input
              type="password"
              className="host-token-input"
              placeholder={t('hostIntegration.token')}
              value={token}
              onChange={(e) => setToken(e.target.value)}
              onKeyDown={handleKeyDown}
              aria-label={t('hostIntegration.token')}
            />
            <button
              type="button"
              className="host-auth-btn"
              onClick={handleAuthenticate}
              disabled={authenticating || !token.trim()}
            >
              {authenticating ? '...' : t('hostIntegration.authenticate')}
            </button>
          </div>
        ) : (
          <div className="host-auth-status">
            <span className="host-auth-badge" data-testid="auth-status">
              ✓ {t('hostIntegration.authenticated')}
            </span>
            <button
              type="button"
              className="host-logout-btn"
              onClick={onLogout}
            >
              {t('hostIntegration.logout')}
            </button>
          </div>
        )}

        {/* Error display */}
        {error && (
          <div className="host-error" role="alert">
            {error}
          </div>
        )}

        {/* PR list (only when authenticated) */}
        {isAuthenticated && (
          <>
            <div className="host-pr-header">
              <span className="host-pr-title">
                {t('hostIntegration.pullRequests')}
              </span>
              <button
                type="button"
                className="host-refresh-btn"
                onClick={onRefreshPrs}
                disabled={loadingPrs}
                aria-label={t('hostIntegration.refreshPrs')}
              >
                ↻
              </button>
            </div>

            {loadingPrs ? (
              <div className="host-loading">{t('hostIntegration.loadingPrs')}</div>
            ) : pullRequests.length === 0 ? (
              <div className="host-empty">{t('hostIntegration.noPullRequests')}</div>
            ) : (
              <ul className="sidebar-list host-pr-list" role="list">
                {pullRequests.map((pr) => (
                  <li key={pr.id} className="sidebar-item host-pr-item">
                    <span
                      className={`host-pr-badge ${prStateBadgeClass(pr.state)}`}
                    >
                      {t(prStateKey(pr.state))}
                    </span>
                    <a
                      className="host-pr-link"
                      href={pr.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      title={pr.title}
                    >
                      #{pr.id} {pr.title}
                    </a>
                  </li>
                ))}
              </ul>
            )}

            {/* Create PR toggle */}
            <button
              type="button"
              className="host-create-pr-toggle"
              onClick={() => setShowCreateForm(!showCreateForm)}
            >
              {showCreateForm ? '−' : '+'} {t('hostIntegration.createPr')}
            </button>

            {/* Create PR form */}
            {showCreateForm && (
              <div className="host-create-pr-form" data-testid="create-pr-form">
                <input
                  type="text"
                  className="host-input"
                  placeholder={t('hostIntegration.prTitle')}
                  value={prTitle}
                  onChange={(e) => setPrTitle(e.target.value)}
                  aria-label={t('hostIntegration.prTitle')}
                />
                <textarea
                  className="host-textarea"
                  placeholder={t('hostIntegration.prDescription')}
                  value={prDescription}
                  onChange={(e) => setPrDescription(e.target.value)}
                  rows={3}
                  aria-label={t('hostIntegration.prDescription')}
                />
                <div className="host-branch-selectors">
                  <select
                    className="host-select"
                    value={sourceBranch}
                    onChange={(e) => setSourceBranch(e.target.value)}
                    aria-label={t('hostIntegration.sourceBranch')}
                  >
                    <option value="">{t('hostIntegration.sourceBranch')}</option>
                    {localBranches.map((b) => (
                      <option key={b} value={b}>{b}</option>
                    ))}
                  </select>
                  <select
                    className="host-select"
                    value={targetBranch}
                    onChange={(e) => setTargetBranch(e.target.value)}
                    aria-label={t('hostIntegration.targetBranch')}
                  >
                    <option value="">{t('hostIntegration.targetBranch')}</option>
                    {localBranches.map((b) => (
                      <option key={b} value={b}>{b}</option>
                    ))}
                  </select>
                </div>
                <button
                  type="button"
                  className="host-create-btn"
                  onClick={handleCreatePr}
                  disabled={creating || !prTitle.trim() || !sourceBranch || !targetBranch}
                >
                  {creating ? t('hostIntegration.creating') : t('hostIntegration.create')}
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </SidebarSection>
  );
}
