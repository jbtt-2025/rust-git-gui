import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { CommitTemplate } from '../../ipc/types';

export interface TemplateSelectorProps {
  templates: CommitTemplate[];
  onSelectTemplate: (content: string) => void;
  onAddTemplate: (template: CommitTemplate) => void;
  onUpdateTemplate: (id: string, partial: Partial<CommitTemplate>) => void;
  onRemoveTemplate: (id: string) => void;
}

export function TemplateSelector({
  templates,
  onSelectTemplate,
  onAddTemplate,
  onUpdateTemplate,
  onRemoveTemplate,
}: TemplateSelectorProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<CommitTemplate | null>(null);
  const [creating, setCreating] = useState(false);
  const [draftName, setDraftName] = useState('');
  const [draftContent, setDraftContent] = useState('');

  const handleSelect = useCallback(
    (template: CommitTemplate) => {
      onSelectTemplate(template.content);
      setOpen(false);
    },
    [onSelectTemplate],
  );

  const handleStartCreate = useCallback(() => {
    setCreating(true);
    setEditing(null);
    setDraftName('');
    setDraftContent('');
  }, []);

  const handleStartEdit = useCallback((template: CommitTemplate) => {
    setEditing(template);
    setCreating(false);
    setDraftName(template.name);
    setDraftContent(template.content);
  }, []);

  const handleSave = useCallback(() => {
    if (!draftName.trim()) return;
    if (editing) {
      onUpdateTemplate(editing.id, { name: draftName, content: draftContent });
    } else {
      onAddTemplate({
        id: crypto.randomUUID(),
        name: draftName,
        content: draftContent,
      });
    }
    setEditing(null);
    setCreating(false);
    setDraftName('');
    setDraftContent('');
  }, [editing, draftName, draftContent, onAddTemplate, onUpdateTemplate]);

  const handleCancel = useCallback(() => {
    setEditing(null);
    setCreating(false);
    setDraftName('');
    setDraftContent('');
  }, []);

  const showForm = creating || editing !== null;

  return (
    <div className="relative">
      <button
        className="text-xs px-2 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        {t('commit.template')}
      </button>

      {open && (
        <div className="absolute bottom-full mb-1 left-0 w-64 bg-gray-800 border border-gray-600 rounded shadow-lg z-50">
          {/* Template list */}
          {!showForm && (
            <>
              <ul className="max-h-40 overflow-auto divide-y divide-gray-700/50" role="listbox">
                {templates.length === 0 && (
                  <li className="px-3 py-2 text-xs text-gray-500 text-center">
                    No templates
                  </li>
                )}
                {templates.map((tpl) => (
                  <li
                    key={tpl.id}
                    className="flex items-center justify-between px-3 py-1.5 hover:bg-gray-700/50 cursor-pointer"
                    role="option"
                    aria-selected={false}
                  >
                    <span
                      className="flex-1 truncate text-xs text-gray-200"
                      onClick={() => handleSelect(tpl)}
                    >
                      {tpl.name}
                    </span>
                    <div className="flex gap-1 ml-2">
                      <button
                        className="text-xs text-blue-400 hover:text-blue-300"
                        onClick={(e) => { e.stopPropagation(); handleStartEdit(tpl); }}
                        aria-label={`Edit ${tpl.name}`}
                      >
                        ✎
                      </button>
                      <button
                        className="text-xs text-red-400 hover:text-red-300"
                        onClick={(e) => { e.stopPropagation(); onRemoveTemplate(tpl.id); }}
                        aria-label={`Delete ${tpl.name}`}
                      >
                        ✕
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
              <div className="border-t border-gray-700 px-3 py-1.5">
                <button
                  className="text-xs text-green-400 hover:text-green-300"
                  onClick={handleStartCreate}
                >
                  + New template
                </button>
              </div>
            </>
          )}

          {/* Create / Edit form */}
          {showForm && (
            <div className="p-3 space-y-2">
              <input
                className="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200 focus:outline-none focus:border-blue-500"
                placeholder="Template name"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                autoFocus
              />
              <textarea
                className="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs text-gray-200 resize-none focus:outline-none focus:border-blue-500"
                placeholder="Template content"
                rows={4}
                value={draftContent}
                onChange={(e) => setDraftContent(e.target.value)}
              />
              <div className="flex justify-end gap-2">
                <button
                  className="text-xs px-2 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
                  onClick={handleCancel}
                >
                  {t('common.cancel')}
                </button>
                <button
                  className="text-xs px-2 py-0.5 rounded bg-blue-600 hover:bg-blue-500 text-white disabled:opacity-50"
                  onClick={handleSave}
                  disabled={!draftName.trim()}
                >
                  {t('common.save')}
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
