import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { BlameLine } from '../../ipc/types';

export interface BlameTimelineProps {
  lines: BlameLine[];
  colorMap: Map<string, number>;
}

const PALETTE = [
  'bg-blue-900/40',
  'bg-green-900/40',
  'bg-purple-900/40',
  'bg-yellow-900/40',
  'bg-red-900/40',
  'bg-cyan-900/40',
  'bg-pink-900/40',
  'bg-orange-900/40',
  'bg-teal-900/40',
  'bg-indigo-900/40',
  'bg-lime-900/40',
  'bg-amber-900/40',
];

/**
 * Timeline visualization showing when each line was last modified.
 * Renders a vertical bar for each line, with width proportional to the
 * line's age relative to the newest and oldest commits in the file.
 */
export function BlameTimeline({ lines, colorMap }: BlameTimelineProps) {
  const { t } = useTranslation();

  const { minDate, maxDate } = useMemo(() => {
    if (lines.length === 0) return { minDate: 0, maxDate: 0 };
    let min = Infinity;
    let max = -Infinity;
    for (const line of lines) {
      if (line.date < min) min = line.date;
      if (line.date > max) max = line.date;
    }
    return { minDate: min, maxDate: max };
  }, [lines]);

  const range = maxDate - minDate;

  return (
    <div className="flex flex-col" role="img" aria-label={t('blame.timeline')}>
      {lines.map((line) => {
        // Normalized age: 0 = oldest, 1 = newest
        const normalized = range > 0 ? (line.date - minDate) / range : 1;
        // Width: newer commits get wider bars (more recent = more prominent)
        const widthPercent = Math.max(10, Math.round(normalized * 100));
        const colorIndex = colorMap.get(line.commit_id) ?? 0;
        const bgClass = PALETTE[colorIndex % PALETTE.length];

        return (
          <div
            key={line.line_number}
            className="h-5 flex items-center"
            title={`${new Date(line.date * 1000).toLocaleDateString()} — ${line.author}`}
          >
            <div
              className={`h-3 rounded-sm ${bgClass}`}
              style={{ width: `${widthPercent}%` }}
            />
          </div>
        );
      })}
    </div>
  );
}
