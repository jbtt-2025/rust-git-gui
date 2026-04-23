import { useTranslation } from 'react-i18next';

export interface CloneOptionsProps {
  recursive: boolean;
  onRecursiveChange: (value: boolean) => void;
}

/**
 * Clone dialog enhancement — checkbox for recursive submodule cloning.
 * Meant to be embedded inside a clone dialog.
 */
export function CloneOptions({ recursive, onRecursiveChange }: CloneOptionsProps) {
  const { t } = useTranslation();

  return (
    <label className="clone-option">
      <input
        type="checkbox"
        className="clone-option-checkbox"
        checked={recursive}
        onChange={(e) => onRecursiveChange(e.target.checked)}
      />
      <span className="clone-option-label">{t('submodule.cloneRecursive')}</span>
    </label>
  );
}
