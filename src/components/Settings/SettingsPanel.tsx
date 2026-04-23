import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { useTranslation } from 'react-i18next';
import { useSettingsStore } from '../../stores/settingsStore';
import { gitApi } from '../../ipc/client';
import { AppearanceSection } from './AppearanceSection';
import { HotkeySection } from './HotkeySection';
import { GitConfigSection } from './GitConfigSection';
import { TemplateSection } from './TemplateSection';
import './SettingsPanel.css';

export interface SettingsPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type Section = 'appearance' | 'shortcuts' | 'gitConfig' | 'templates';

const SECTIONS: Section[] = ['appearance', 'shortcuts', 'gitConfig', 'templates'];

export function SettingsPanel({ open, onOpenChange }: SettingsPanelProps) {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<Section>('appearance');
  const settings = useSettingsStore((s) => s.settings);
  const dirty = useSettingsStore((s) => s.dirty);
  const markClean = useSettingsStore((s) => s.markClean);

  const handleSave = async () => {
    try {
      await gitApi.saveAppSettings('settings.json', settings);
      markClean();
    } catch {
      // save error handled upstream
    }
  };

  const renderSection = () => {
    switch (activeSection) {
      case 'appearance':
        return <AppearanceSection />;
      case 'shortcuts':
        return <HotkeySection />;
      case 'gitConfig':
        return <GitConfigSection />;
      case 'templates':
        return <TemplateSection />;
    }
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="settings-overlay" />
        <Dialog.Content className="settings-content" aria-describedby={undefined}>
          <div className="settings-header">
            <Dialog.Title className="settings-title">{t('settings.title')}</Dialog.Title>
            <Dialog.Close asChild>
              <button className="settings-close-btn" type="button" aria-label={t('common.close')}>
                ✕
              </button>
            </Dialog.Close>
          </div>

          <div className="settings-body">
            <nav className="settings-nav" aria-label="Settings sections">
              {SECTIONS.map((section) => (
                <button
                  key={section}
                  className={`settings-nav-item${activeSection === section ? ' settings-nav-item--active' : ''}`}
                  type="button"
                  onClick={() => setActiveSection(section)}
                >
                  {t(`settings.${section}`)}
                </button>
              ))}
            </nav>
            <div className="settings-section">{renderSection()}</div>
          </div>

          <div className="settings-footer">
            <button
              className="settings-save-btn"
              type="button"
              disabled={!dirty}
              onClick={handleSave}
            >
              {t('common.save')}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
