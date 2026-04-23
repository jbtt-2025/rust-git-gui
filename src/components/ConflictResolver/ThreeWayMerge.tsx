import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface ThreeWayMergeProps {
  filePath: string;
  oursContent: string;
  theirsContent: string;
  initialMergeContent: string;
  onMergeContentChange: (filePath: string, content: string) => void;
}

export function ThreeWayMerge({
  filePath,
  oursContent,
  theirsContent,
  initialMergeContent,
  onMergeContentChange,
}: ThreeWayMergeProps) {
  const { t } = useTranslation();
  const [mergeContent, setMergeContent] = useState(initialMergeContent);

  const handleChange = (value: string) => {
    setMergeContent(value);
    onMergeContentChange(filePath, value);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 grid grid-cols-3 gap-px bg-gray-700 min-h-0">
        {/* Ours (local) */}
        <div className="flex flex-col bg-gray-900 min-h-0">
          <div className="px-3 py-1 bg-gray-800 text-xs font-medium text-blue-400 border-b border-gray-700">
            {t('conflict.ours')}
          </div>
          <pre className="flex-1 overflow-auto p-2 text-xs text-gray-300 whitespace-pre-wrap font-mono">
            {oursContent}
          </pre>
        </div>

        {/* Theirs (remote) */}
        <div className="flex flex-col bg-gray-900 min-h-0">
          <div className="px-3 py-1 bg-gray-800 text-xs font-medium text-orange-400 border-b border-gray-700">
            {t('conflict.theirs')}
          </div>
          <pre className="flex-1 overflow-auto p-2 text-xs text-gray-300 whitespace-pre-wrap font-mono">
            {theirsContent}
          </pre>
        </div>

        {/* Merge result (editable) */}
        <div className="flex flex-col bg-gray-900 min-h-0">
          <div className="px-3 py-1 bg-gray-800 text-xs font-medium text-green-400 border-b border-gray-700">
            {t('conflict.merge')}
          </div>
          <textarea
            className="flex-1 overflow-auto p-2 text-xs text-gray-300 bg-gray-900 resize-none font-mono focus:outline-none focus:ring-1 focus:ring-blue-500"
            value={mergeContent}
            onChange={(e) => handleChange(e.target.value)}
            aria-label={t('conflict.merge')}
            spellCheck={false}
          />
        </div>
      </div>
    </div>
  );
}
