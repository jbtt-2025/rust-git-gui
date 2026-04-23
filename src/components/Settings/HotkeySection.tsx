import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettingsStore } from '../../stores/settingsStore';

/** Format a KeyboardEvent into a human-readable shortcut string. */
function formatKeyCombo(e: KeyboardEvent): string | null {
  // Ignore bare modifier keys
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return null;

  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');

  let key = e.key;
  if (key === ' ') key = 'Space';
  else if (key.length === 1) key = key.toUpperCase();
  else if (key === 'Backquote' || key === '`') key = '`';

  parts.push(key);
  return parts.join('+');
}

export function HotkeySection() {
  const { t } = useTranslation();
  const hotkeys = useSettingsStore((s) => s.settings.hotkeys);
  const setHotkey = useSettingsStore((s) => s.setHotkey);

  const [recordingAction, setRecordingAction] = useState<string | null>(null);
  const [conflict, setConflict] = useState<{ action: string; existingAction: string } | null>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!recordingAction) return;
      e.preventDefault();
      e.stopPropagation();

      const combo = formatKeyCombo(e);
      if (!combo) return;

      // Check for conflicts
      const conflicting = Object.entries(hotkeys).find(
        ([action, binding]) => binding === combo && action !== recordingAction,
      );

      if (conflicting) {
        setConflict({ action: recordingAction, existingAction: conflicting[0] });
      } else {
        setConflict(null);
        setHotkey(recordingAction, combo);
        setRecordingAction(null);
      }
    },
    [recordingAction, hotkeys, setHotkey],
  );

  useEffect(() => {
    if (recordingAction) {
      window.addEventListener('keydown', handleKeyDown, true);
      return () => window.removeEventListener('keydown', handleKeyDown, true);
    }
  }, [recordingAction, handleKeyDown]);

  const startRecording = (action: string) => {
    setConflict(null);
    setRecordingAction(action);
  };

  const cancelRecording = () => {
    setRecordingAction(null);
    setConflict(null);
  };

  return (
    <div>
      <h3 className="settings-section-title">{t('settings.shortcuts')}</h3>
      <table className="settings-hotkey-table">
        <thead>
          <tr>
            <th>Action</th>
            <th>Shortcut</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {Object.entries(hotkeys).map(([action, binding]) => (
            <tr key={action}>
              <td>{action}</td>
              <td>
                {recordingAction === action ? (
                  <span className="settings-hotkey-recording">
                    {t('settings.recording')}
                  </span>
                ) : (
                  <span className="settings-hotkey-binding">{binding}</span>
                )}
                {conflict && conflict.action === action && (
                  <div className="settings-hotkey-conflict">
                    {t('settings.hotkeyConflict', { action: conflict.existingAction })}
                  </div>
                )}
              </td>
              <td>
                {recordingAction === action ? (
                  <button
                    className="settings-hotkey-edit-btn"
                    type="button"
                    onClick={cancelRecording}
                  >
                    {t('common.cancel')}
                  </button>
                ) : (
                  <button
                    className="settings-hotkey-edit-btn"
                    type="button"
                    onClick={() => startRecording(action)}
                  >
                    Edit
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
