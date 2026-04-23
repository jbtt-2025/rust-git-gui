import { useTranslation } from 'react-i18next';

export interface SubmoduleUpdateBannerProps {
  show: boolean;
  submodules: string[];
  onUpdateAll: () => void;
  onDismiss: () => void;
}

/**
 * Notification banner shown after pull when submodule refs changed.
 * Displays at the top of the sidebar with "Update All" and "Dismiss" actions.
 */
export function SubmoduleUpdateBanner({
  show,
  submodules,
  onUpdateAll,
  onDismiss,
}: SubmoduleUpdateBannerProps) {
  const { t } = useTranslation();

  if (!show || submodules.length === 0) return null;

  return (
    <div className="submodule-update-banner" role="alert">
      <p className="submodule-update-banner-text">
        {t('submodule.updateBanner')}
      </p>
      <div className="submodule-update-banner-actions">
        <button
          type="button"
          className="submodule-update-banner-btn submodule-update-banner-btn--primary"
          onClick={onUpdateAll}
        >
          {t('submodule.updateBannerAction')}
        </button>
        <button
          type="button"
          className="submodule-update-banner-btn"
          onClick={onDismiss}
        >
          {t('submodule.dismiss')}
        </button>
      </div>
    </div>
  );
}
