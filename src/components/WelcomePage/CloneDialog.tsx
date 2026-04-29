import { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import * as Dialog from '@radix-ui/react-dialog';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { validateCloneForm } from './validateCloneForm';
import type { ProgressEvent } from '../../ipc/types';

export interface CloneDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onClone: (url: string, path: string, recursive: boolean) => Promise<void>;
}

interface CloneFormState {
  url: string;
  path: string;
  recursive: boolean;
  errors: { url?: string; path?: string };
  cloning: boolean;
  progress: { current: number; total: number | null; message: string | null } | null;
  cloneError: string | null;
}

const initialFormState: CloneFormState = {
  url: '',
  path: '',
  recursive: false,
  errors: {},
  cloning: false,
  progress: null,
  cloneError: null,
};

/**
 * Modal dialog for cloning a remote Git repository.
 * Includes URL input, local path selector with Browse button,
 * recursive submodule checkbox, progress bar, and inline error display.
 */
export function CloneDialog({ open: isOpen, onOpenChange, onClone }: CloneDialogProps) {
  const { t } = useTranslation();
  const [form, setForm] = useState<CloneFormState>(initialFormState);

  // Reset form state when dialog closes
  useEffect(() => {
    if (!isOpen) {
      setForm(initialFormState);
    }
  }, [isOpen]);

  // Listen for operation-progress events during cloning
  useEffect(() => {
    if (!form.cloning) return;

    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen<ProgressEvent>('operation-progress', (event) => {
        const { current, total, message } = event.payload;
        setForm((prev) => ({
          ...prev,
          progress: { current, total, message },
        }));
      });
    };

    setupListener();

    return () => {
      unlisten?.();
    };
  }, [form.cloning]);

  const handleBrowse = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      setForm((prev) => ({
        ...prev,
        path: selected as string,
        errors: { ...prev.errors, path: undefined },
      }));
    }
  }, []);

  const handleSubmit = useCallback(async () => {
    const errors = validateCloneForm(form.url, form.path);
    if (errors.url || errors.path) {
      setForm((prev) => ({ ...prev, errors }));
      return;
    }

    setForm((prev) => ({
      ...prev,
      errors: {},
      cloning: true,
      cloneError: null,
      progress: null,
    }));

    try {
      await onClone(form.url, form.path, form.recursive);
      // On success the parent will close the dialog via onOpenChange
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setForm((prev) => ({
        ...prev,
        cloning: false,
        progress: null,
        cloneError: message,
      }));
    }
  }, [form.url, form.path, form.recursive, onClone]);

  const progressPercent =
    form.progress && form.progress.total
      ? Math.round((form.progress.current / form.progress.total) * 100)
      : null;

  return (
    <Dialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50" />
        <Dialog.Content
          className="fixed left-1/2 top-1/2 w-full max-w-md -translate-x-1/2 -translate-y-1/2 rounded-lg border border-neutral-600 bg-neutral-800 p-6 shadow-xl focus:outline-none"
          aria-describedby="clone-dialog-description"
        >
          <Dialog.Title className="text-lg font-semibold text-neutral-200">
            {t('welcome.cloneTitle')}
          </Dialog.Title>

          <p id="clone-dialog-description" className="sr-only">
            {t('welcome.subtitle')}
          </p>

          <div className="mt-4 flex flex-col gap-4">
            {/* URL input */}
            <div className="flex flex-col gap-1">
              <label htmlFor="clone-url" className="text-sm text-neutral-400">
                {t('welcome.cloneUrl')}
              </label>
              <input
                id="clone-url"
                type="text"
                value={form.url}
                onChange={(e) =>
                  setForm((prev) => ({
                    ...prev,
                    url: e.target.value,
                    errors: { ...prev.errors, url: undefined },
                  }))
                }
                disabled={form.cloning}
                placeholder="https://github.com/user/repo.git"
                aria-required="true"
                aria-invalid={!!form.errors.url}
                className={`w-full rounded-md border px-3 py-2 text-sm text-neutral-200 placeholder-neutral-500 focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                  form.errors.url
                    ? 'border-red-500 bg-neutral-800'
                    : 'border-neutral-600 bg-neutral-800'
                }`}
              />
              {form.errors.url && (
                <span className="text-xs text-red-400" role="alert">
                  {t(form.errors.url)}
                </span>
              )}
            </div>

            {/* Local path input with Browse button */}
            <div className="flex flex-col gap-1">
              <label htmlFor="clone-path" className="text-sm text-neutral-400">
                {t('welcome.clonePath')}
              </label>
              <div className="flex gap-2">
                <input
                  id="clone-path"
                  type="text"
                  value={form.path}
                  onChange={(e) =>
                    setForm((prev) => ({
                      ...prev,
                      path: e.target.value,
                      errors: { ...prev.errors, path: undefined },
                    }))
                  }
                  disabled={form.cloning}
                  aria-required="true"
                  aria-invalid={!!form.errors.path}
                  className={`flex-1 rounded-md border px-3 py-2 text-sm text-neutral-200 placeholder-neutral-500 focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                    form.errors.path
                      ? 'border-red-500 bg-neutral-800'
                      : 'border-neutral-600 bg-neutral-800'
                  }`}
                />
                <button
                  type="button"
                  onClick={handleBrowse}
                  disabled={form.cloning}
                  aria-label={t('welcome.cloneBrowse')}
                  className="rounded-md bg-neutral-700 px-3 py-2 text-sm font-medium text-neutral-200 hover:bg-neutral-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                >
                  {t('welcome.cloneBrowse')}
                </button>
              </div>
              {form.errors.path && (
                <span className="text-xs text-red-400" role="alert">
                  {t(form.errors.path)}
                </span>
              )}
            </div>

            {/* Recursive submodules checkbox */}
            <label className="flex items-center gap-2 text-sm text-neutral-300">
              <input
                type="checkbox"
                checked={form.recursive}
                onChange={(e) =>
                  setForm((prev) => ({ ...prev, recursive: e.target.checked }))
                }
                disabled={form.cloning}
                className="h-4 w-4 rounded border-neutral-600 bg-neutral-800 text-blue-500 focus:ring-2 focus:ring-blue-500"
              />
              {t('welcome.cloneRecursive')}
            </label>

            {/* Progress bar */}
            {form.cloning && (
              <div className="flex flex-col gap-1">
                <div className="h-2 w-full overflow-hidden rounded-full bg-neutral-700">
                  <div
                    className="h-full rounded-full bg-blue-500 transition-all duration-200"
                    style={{
                      width: progressPercent != null ? `${progressPercent}%` : '100%',
                    }}
                    role="progressbar"
                    aria-valuenow={progressPercent ?? undefined}
                    aria-valuemin={0}
                    aria-valuemax={100}
                  />
                </div>
                <span className="text-xs text-neutral-400">
                  {progressPercent != null
                    ? t('welcome.cloneProgress', { percent: progressPercent })
                    : form.progress?.message ?? t('common.loading')}
                </span>
              </div>
            )}

            {/* Clone error message */}
            {form.cloneError && (
              <div className="rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-400" role="alert">
                {t('welcome.cloneError', { message: form.cloneError })}
              </div>
            )}
          </div>

          {/* Action buttons */}
          <div className="mt-6 flex justify-end gap-3">
            <Dialog.Close asChild>
              <button
                type="button"
                disabled={form.cloning}
                className="rounded-md bg-neutral-700 px-4 py-2 text-sm font-medium text-neutral-200 hover:bg-neutral-600 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
              >
                {t('common.cancel')}
              </button>
            </Dialog.Close>
            <button
              type="button"
              onClick={handleSubmit}
              disabled={form.cloning}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
            >
              {t('welcome.cloneStart')}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
