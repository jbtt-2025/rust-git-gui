import { useTranslation } from 'react-i18next';

export interface ConflictFileItem {
  path: string;
  resolved: boolean;
}

export interface ConflictFileListProps {
  files: ConflictFileItem[];
  selectedFile: string | null;
  onSelectFile: (path: string) => void;
  onMarkResolved: (path: string) => void;
}

export function ConflictFileList({
  files,
  selectedFile,
  onSelectFile,
  onMarkResolved,
}: ConflictFileListProps) {
  const { t } = useTranslation();

  return (
    <ul className="divide-y divide-gray-700/50" role="listbox" aria-label={t('conflict.title')}>
      {files.map((file) => (
        <li
          key={file.path}
          className={`flex items-center gap-2 px-3 py-1.5 cursor-pointer hover:bg-gray-700/50 ${
            selectedFile === file.path ? 'bg-blue-900/30' : ''
          }`}
          onClick={() => onSelectFile(file.path)}
          role="option"
          aria-selected={selectedFile === file.path}
        >
          <span
            className={`w-2 h-2 rounded-full flex-shrink-0 ${
              file.resolved ? 'bg-green-500' : 'bg-red-500'
            }`}
            aria-label={file.resolved ? 'resolved' : 'unresolved'}
          />
          <span className="flex-1 truncate text-xs">{file.path}</span>
          {!file.resolved && (
            <button
              className="text-xs px-1.5 py-0.5 rounded bg-green-700 hover:bg-green-600 text-white"
              onClick={(e) => {
                e.stopPropagation();
                onMarkResolved(file.path);
              }}
            >
              {t('conflict.resolved')}
            </button>
          )}
        </li>
      ))}
    </ul>
  );
}
