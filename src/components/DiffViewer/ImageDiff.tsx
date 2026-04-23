import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface ImageDiffProps {
  path: string;
  oldPath?: string | null;
  status: string;
}

type ImageDiffMode = 'side-by-side' | 'overlay';

/**
 * Image comparison component for binary image files.
 * Shows old and new images side by side or with an overlay slider.
 */
export function ImageDiff({ path, oldPath, status }: ImageDiffProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<ImageDiffMode>('side-by-side');
  const [opacity, setOpacity] = useState(0.5);

  const oldSrc = oldPath ?? path;
  const newSrc = path;
  const isAdded = status === 'Added';
  const isDeleted = status === 'Deleted';

  return (
    <div className="p-4">
      {/* Mode toggle */}
      <div className="flex gap-2 mb-3">
        <button
          className={`px-3 py-1 text-xs rounded ${mode === 'side-by-side' ? 'bg-blue-600 text-white' : 'bg-gray-700 text-gray-300'}`}
          onClick={() => setMode('side-by-side')}
        >
          {t('diff.split')}
        </button>
        <button
          className={`px-3 py-1 text-xs rounded ${mode === 'overlay' ? 'bg-blue-600 text-white' : 'bg-gray-700 text-gray-300'}`}
          onClick={() => setMode('overlay')}
        >
          Overlay
        </button>
      </div>

      {mode === 'side-by-side' ? (
        <div className="grid grid-cols-2 gap-4">
          {!isAdded && (
            <div className="border border-red-800/50 rounded p-2">
              <div className="text-xs text-red-400 mb-1">Old</div>
              <img
                src={oldSrc}
                alt="Old version"
                className="max-w-full h-auto"
              />
            </div>
          )}
          {isAdded && <div className="border border-gray-700 rounded p-2 flex items-center justify-center text-gray-500 text-sm">No previous version</div>}
          {!isDeleted && (
            <div className="border border-green-800/50 rounded p-2">
              <div className="text-xs text-green-400 mb-1">New</div>
              <img
                src={newSrc}
                alt="New version"
                className="max-w-full h-auto"
              />
            </div>
          )}
          {isDeleted && <div className="border border-gray-700 rounded p-2 flex items-center justify-center text-gray-500 text-sm">File deleted</div>}
        </div>
      ) : (
        <div className="relative inline-block">
          {!isAdded && (
            <img src={oldSrc} alt="Old version" className="max-w-full h-auto" />
          )}
          {!isDeleted && (
            <img
              src={newSrc}
              alt="New version"
              className="absolute top-0 left-0 max-w-full h-auto"
              style={{ opacity }}
            />
          )}
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={opacity}
            onChange={(e) => setOpacity(Number(e.target.value))}
            className="mt-2 w-full"
            aria-label="Overlay opacity"
          />
        </div>
      )}
    </div>
  );
}
