import { useTranslation } from 'react-i18next';
import { useUiStore } from '../../stores/uiStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { applyTheme } from '../../themes';
import { changeLanguage } from '../../i18n';
import type { ThemeMode } from '../../ipc/types';

export function AppearanceSection() {
  const { t } = useTranslation();
  const theme = useUiStore((s) => s.theme);
  const setTheme = useUiStore((s) => s.setTheme);
  const settings = useSettingsStore((s) => s.settings);
  const setFontFamily = useSettingsStore((s) => s.setFontFamily);
  const setFontSize = useSettingsStore((s) => s.setFontSize);
  const setLanguage = useSettingsStore((s) => s.setLanguage);
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  const handleThemeChange = (mode: ThemeMode) => {
    setTheme(mode);
    updateSettings({ theme: mode });
    applyTheme(mode);
  };

  const handleLanguageChange = (lang: string) => {
    setLanguage(lang);
    changeLanguage(lang);
  };

  const handleFontSizeChange = (value: string) => {
    const n = parseInt(value, 10);
    if (!isNaN(n) && n >= 10 && n <= 24) {
      setFontSize(n);
    }
  };

  return (
    <div>
      <h3 className="settings-section-title">{t('settings.appearance')}</h3>

      {/* Theme */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.theme')}</label>
        <div className="settings-radio-group">
          {(['Light', 'Dark', 'System'] as ThemeMode[]).map((mode) => (
            <label key={mode} className="settings-radio-label">
              <input
                type="radio"
                name="theme"
                checked={theme === mode}
                onChange={() => handleThemeChange(mode)}
              />
              {t(`settings.theme${mode}`)}
            </label>
          ))}
        </div>
      </div>

      {/* Language */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.language')}</label>
        <select
          className="settings-select"
          value={settings.language}
          onChange={(e) => handleLanguageChange(e.target.value)}
          style={{ width: 200 }}
        >
          <option value="en">English</option>
          <option value="zh_CN">简体中文</option>
          <option value="ja">日本語</option>
        </select>
      </div>

      {/* Font family */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.font')}</label>
        <input
          className="settings-input"
          type="text"
          value={settings.font_family}
          onChange={(e) => setFontFamily(e.target.value)}
          style={{ width: 250 }}
        />
      </div>

      {/* Font size */}
      <div className="settings-field">
        <label className="settings-label">{t('settings.fontSize')}</label>
        <input
          className="settings-input settings-input--number"
          type="number"
          min={10}
          max={24}
          value={settings.font_size}
          onChange={(e) => handleFontSizeChange(e.target.value)}
        />
      </div>
    </div>
  );
}
