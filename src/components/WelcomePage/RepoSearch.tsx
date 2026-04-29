import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';

export interface RepoSearchProps {
  value: string;
  onChange: (value: string) => void;
  onClear: () => void;
}

/**
 * Search input for filtering the recent repositories list.
 * Supports Escape key to clear the search and restore the full list.
 */
export function RepoSearch({ value, onChange, onClear }: RepoSearchProps) {
  const { t } = useTranslation();

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClear();
      }
    },
    [onClear],
  );

  return (
    <div className="relative w-full">
      <input
        type="search"
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        placeholder={t('welcome.searchPlaceholder')}
        aria-label={t('welcome.searchPlaceholder')}
        className="w-full rounded-md border border-neutral-600 bg-neutral-800 px-3 py-2 text-sm text-neutral-200 placeholder-neutral-500 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>
  );
}
