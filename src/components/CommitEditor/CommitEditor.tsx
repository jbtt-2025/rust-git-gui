import { useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { TabId, CommitTemplate } from '../../ipc/types';
import { checkCommitTitle } from './commitTitleCheck';
import { TemplateSelector } from './TemplateSelector';

export interface CommitEditorProps {
  tabId: TabId;
  hasStagedChanges: boolean;
  templates: CommitTemplate[];
  onCommit: (tabId: TabId, message: string) => void;
  onAmendCommit: (tabId: TabId, message: string) => void;
  onAddTemplate: (template: CommitTemplate) => void;
  onUpdateTemplate: (id: string, partial: Partial<CommitTemplate>) => void;
  onRemoveTemplate: (id: string) => void;
}

export function CommitEditor({
  tabId,
  hasStagedChanges,
  templates,
  onCommit,
  onAmendCommit,
  onAddTemplate,
  onUpdateTemplate,
  onRemoveTemplate,
}: CommitEditorProps) {
  const { t } = useTranslation();
  const [message, setMessage] = useState('');
  const [amend, setAmend] = useState(false);

  // Extract the title line (first line before newline)
  const titleLine = useMemo(() => {
    const idx = message.indexOf('\n');
    return idx === -1 ? message : message.substring(0, idx);
  }, [message]);

  const titleCheck = useMemo(() => checkCommitTitle(titleLine), [titleLine]);

  const canCommit = message.trim().length > 0 && (hasStagedChanges || amend);

  const handleCommit = useCallback(() => {
    if (!canCommit) return;
    if (amend) {
      onAmendCommit(tabId, message);
    } else {
      onCommit(tabId, message);
    }
    setMessage('');
    setAmend(false);
  }, [canCommit, amend, tabId, message, onCommit, onAmendCommit]);

  const handleSelectTemplate = useCallback((content: string) => {
    setMessage(content);
  }, []);

  return (
    <div className="flex flex-col gap-2 p-3 bg-gray-800 border-t border-gray-700">
      {/* Header row: title + char count */}
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-gray-300">{t('commit.title')}</span>
        <span
          className={`text-xs ${titleCheck.warning ? 'text-red-400 font-semibold' : 'text-gray-500'}`}
          aria-live="polite"
        >
          {titleCheck.length}/72
        </span>
      </div>

      {/* Warning message */}
      {titleCheck.warning && (
        <div className="text-xs text-red-400" role="alert">
          {t('commit.titleTooLong')}
        </div>
      )}

      {/* Message textarea */}
      <textarea
        className={`w-full bg-gray-900 rounded px-2 py-1.5 text-xs text-gray-200 resize-none focus:outline-none ${
          titleCheck.warning
            ? 'border border-red-500 focus:border-red-400'
            : 'border border-gray-600 focus:border-blue-500'
        }`}
        placeholder={t('commit.message')}
        rows={4}
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        aria-label={t('commit.message')}
      />

      {/* Bottom row: amend + template + commit button */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {/* Amend checkbox */}
          <label className="flex items-center gap-1 text-xs text-gray-300 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={amend}
              onChange={(e) => setAmend(e.target.checked)}
              className="rounded border-gray-600 bg-gray-900 text-blue-500 focus:ring-blue-500 focus:ring-offset-0"
            />
            {t('commit.amend')}
          </label>

          {/* Template selector */}
          <TemplateSelector
            templates={templates}
            onSelectTemplate={handleSelectTemplate}
            onAddTemplate={onAddTemplate}
            onUpdateTemplate={onUpdateTemplate}
            onRemoveTemplate={onRemoveTemplate}
          />
        </div>

        {/* Commit button */}
        <div className="relative group">
          <button
            className="text-xs px-3 py-1 rounded bg-green-600 hover:bg-green-500 text-white font-medium disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={!canCommit}
            onClick={handleCommit}
          >
            {amend ? t('commit.amend') : t('commit.title')}
          </button>
          {/* Tooltip when staging is empty and not amending */}
          {!hasStagedChanges && !amend && (
            <div className="absolute bottom-full mb-1 right-0 hidden group-hover:block bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-yellow-400 whitespace-nowrap shadow-lg">
              {t('commit.emptyStaging')}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
