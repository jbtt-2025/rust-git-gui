import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useRepoStore } from '../../stores/repoStore';
import { gitApi } from '../../ipc/client';
import type { GitConfig } from '../../ipc/types';

const EMPTY_CONFIG: GitConfig = {
  user_name: null,
  user_email: null,
  default_branch: null,
  merge_strategy: null,
};

export function GitConfigSection() {
  const { t } = useTranslation();
  const activeTabId = useRepoStore((s) => s.activeTabId);

  const [level, setLevel] = useState<'local' | 'global'>('local');
  const [config, setConfig] = useState<GitConfig>(EMPTY_CONFIG);
  const [saving, setSaving] = useState(false);

  const loadConfig = useCallback(async () => {
    if (!activeTabId) return;
    try {
      const cfg = await gitApi.getGitConfig(activeTabId, level);
      setConfig(cfg);
    } catch {
      setConfig(EMPTY_CONFIG);
    }
  }, [activeTabId, level]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSave = async () => {
    if (!activeTabId) return;
    setSaving(true);
    try {
      await gitApi.setGitConfig(activeTabId, level, config);
    } finally {
      setSaving(false);
    }
  };

  const update = (field: keyof GitConfig, value: string) => {
    setConfig((prev) => ({ ...prev, [field]: value || null }));
  };

  return (
    <div>
      <h3 className="settings-section-title">{t('settings.gitConfig')}</h3>

      {/* Level toggle */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.configLevel')}</label>
        <div className="settings-toggle-group">
          <button
            className={`settings-toggle-btn${level === 'local' ? ' settings-toggle-btn--active' : ''}`}
            type="button"
            onClick={() => setLevel('local')}
          >
            {t('settings.local')}
          </button>
          <button
            className={`settings-toggle-btn${level === 'global' ? ' settings-toggle-btn--active' : ''}`}
            type="button"
            onClick={() => setLevel('global')}
          >
            {t('settings.global')}
          </button>
        </div>
      </div>

      {/* user.name */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.userName')}</label>
        <input
          className="settings-input"
          type="text"
          value={config.user_name ?? ''}
          onChange={(e) => update('user_name', e.target.value)}
          style={{ width: 300 }}
        />
      </div>

      {/* user.email */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.userEmail')}</label>
        <input
          className="settings-input"
          type="text"
          value={config.user_email ?? ''}
          onChange={(e) => update('user_email', e.target.value)}
          style={{ width: 300 }}
        />
      </div>

      {/* default branch */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.defaultBranch')}</label>
        <input
          className="settings-input"
          type="text"
          value={config.default_branch ?? ''}
          onChange={(e) => update('default_branch', e.target.value)}
          style={{ width: 200 }}
        />
      </div>

      {/* merge strategy */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.mergeStrategy')}</label>
        <select
          className="settings-select"
          value={config.merge_strategy ?? ''}
          onChange={(e) => update('merge_strategy', e.target.value)}
          style={{ width: 200 }}
        >
          <option value="">—</option>
          <option value="recursive">recursive</option>
          <option value="ort">ort</option>
          <option value="resolve">resolve</option>
          <option value="octopus">octopus</option>
          <option value="ours">ours</option>
        </select>
      </div>

      <button
        className="settings-save-btn"
        type="button"
        disabled={saving || !activeTabId}
        onClick={handleSave}
      >
        {t('common.save')}
      </button>
    </div>
  );
}
