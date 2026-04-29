import { useTranslation } from 'react-i18next';

export interface ActionBarProps {
  onOpen: () => void;
  onClone: () => void;
  onInit: () => void;
}

/**
 * Horizontal bar of three action buttons: Open, Clone, and Create.
 * Each button includes an inline SVG icon and a translated text label.
 */
export function ActionBar({ onOpen, onClone, onInit }: ActionBarProps) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center justify-center gap-4" role="toolbar" aria-label={t('welcome.subtitle')}>
      {/* Open button */}
      <button
        type="button"
        onClick={onOpen}
        aria-label={t('welcome.open')}
        className="flex items-center gap-2 rounded-md bg-neutral-700 px-4 py-2 text-sm font-medium text-neutral-200 hover:bg-neutral-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
      >
        {/* Folder-open icon */}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-4 w-4"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fillRule="evenodd"
            d="M2 6a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1H8a3 3 0 00-2.83 2H2V6z"
            clipRule="evenodd"
          />
          <path d="M2 13.692V12a1 1 0 011-1h12a1 1 0 01.95.68l1.4 4.2A1 1 0 0116.4 17H3.6a1 1 0 01-.95-1.31l1.35-4z" />
        </svg>
        {t('welcome.open')}
      </button>

      {/* Clone button */}
      <button
        type="button"
        onClick={onClone}
        aria-label={t('welcome.clone')}
        className="flex items-center gap-2 rounded-md bg-neutral-700 px-4 py-2 text-sm font-medium text-neutral-200 hover:bg-neutral-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
      >
        {/* Download / arrow-down icon */}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-4 w-4"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fillRule="evenodd"
            d="M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm3.293-7.707a1 1 0 011.414 0L9 10.586V3a1 1 0 112 0v7.586l1.293-1.293a1 1 0 111.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z"
            clipRule="evenodd"
          />
        </svg>
        {t('welcome.clone')}
      </button>

      {/* Create button */}
      <button
        type="button"
        onClick={onInit}
        aria-label={t('welcome.init')}
        className="flex items-center gap-2 rounded-md bg-neutral-700 px-4 py-2 text-sm font-medium text-neutral-200 hover:bg-neutral-600 focus:outline-none focus:ring-2 focus:ring-blue-500"
      >
        {/* Plus icon */}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-4 w-4"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fillRule="evenodd"
            d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z"
            clipRule="evenodd"
          />
        </svg>
        {t('welcome.init')}
      </button>
    </div>
  );
}
