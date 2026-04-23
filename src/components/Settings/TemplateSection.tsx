import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettingsStore } from '../../stores/settingsStore';

export function TemplateSection() {
  const { t } = useTranslation();
  const templates = useSettingsStore((s) => s.settings.commit_templates);
  const addCommitTemplate = useSettingsStore((s) => s.addCommitTemplate);
  const updateCommitTemplate = useSettingsStore((s) => s.updateCommitTemplate);
  const removeCommitTemplate = useSettingsStore((s) => s.removeCommitTemplate);

  const [adding, setAdding] = useState(false);
  const [newName, setNewName] = useState('');
  const [newContent, setNewContent] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);

  const handleAdd = () => {
    if (!newName.trim()) return;
    addCommitTemplate({
      id: `tpl-${Date.now()}`,
      name: newName.trim(),
      content: newContent,
    });
    setNewName('');
    setNewContent('');
    setAdding(false);
  };

  const handleDelete = (id: string) => {
    removeCommitTemplate(id);
    setDeleteConfirmId(null);
  };

  return (
    <div>
      <h3 className="settings-section-title">{t('settings.templates')}</h3>

      {templates.map((tpl) => (
        <div key={tpl.id} className="settings-template-item">
          <div className="settings-template-header">
            {editingId === tpl.id ? (
              <input
                className="settings-input"
                type="text"
                value={tpl.name}
                onChange={(e) => updateCommitTemplate(tpl.id, { name: e.target.value })}
                style={{ width: 200 }}
              />
            ) : (
              <span className="settings-template-name">{tpl.name}</span>
            )}
            <div className="settings-template-actions">
              {editingId === tpl.id ? (
                <button
                  className="settings-template-btn"
                  type="button"
                  onClick={() => setEditingId(null)}
                >
                  {t('common.save')}
                </button>
              ) : (
                <button
                  className="settings-template-btn"
                  type="button"
                  onClick={() => setEditingId(tpl.id)}
                >
                  Edit
                </button>
              )}
              {deleteConfirmId === tpl.id ? (
                <>
                  <button
                    className="settings-template-btn settings-template-btn--danger"
                    type="button"
                    onClick={() => handleDelete(tpl.id)}
                  >
                    {t('common.confirm')}
                  </button>
                  <button
                    className="settings-template-btn"
                    type="button"
                    onClick={() => setDeleteConfirmId(null)}
                  >
                    {t('common.cancel')}
                  </button>
                </>
              ) : (
                <button
                  className="settings-template-btn settings-template-btn--danger"
                  type="button"
                  onClick={() => setDeleteConfirmId(tpl.id)}
                >
                  {t('common.delete')}
                </button>
              )}
            </div>
          </div>
          {editingId === tpl.id && (
            <textarea
              className="settings-template-textarea"
              value={tpl.content}
              onChange={(e) => updateCommitTemplate(tpl.id, { content: e.target.value })}
            />
          )}
        </div>
      ))}

      {adding ? (
        <div className="settings-template-item">
          <div className="settings-field">
            <label className="settings-label">{t('settings.templateName')}</label>
            <input
              className="settings-input"
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              style={{ width: 250 }}
            />
          </div>
          <div className="settings-field">
            <label className="settings-label">{t('settings.templateContent')}</label>
            <textarea
              className="settings-template-textarea"
              value={newContent}
              onChange={(e) => setNewContent(e.target.value)}
            />
          </div>
          <div className="settings-template-actions">
            <button className="settings-save-btn" type="button" onClick={handleAdd}>
              {t('common.save')}
            </button>
            <button
              className="settings-template-btn"
              type="button"
              onClick={() => setAdding(false)}
            >
              {t('common.cancel')}
            </button>
          </div>
        </div>
      ) : (
        <button className="settings-add-btn" type="button" onClick={() => setAdding(true)}>
          + {t('settings.addTemplate')}
        </button>
      )}
    </div>
  );
}
