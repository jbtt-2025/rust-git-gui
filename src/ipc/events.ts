// Event listener wrappers for Tauri backend events
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { FileChangeEvent, ProgressEvent } from './types';

/** Listen for file system change events from the backend FileWatcher. */
export function onFileChanged(
  callback: (payload: FileChangeEvent) => void,
): Promise<UnlistenFn> {
  return listen<FileChangeEvent>('file-changed', (event) =>
    callback(event.payload),
  );
}

/** Listen for operation progress events (clone, fetch, push, submodule-update, etc.). */
export function onProgress(
  callback: (payload: ProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ProgressEvent>('operation-progress', (event) =>
    callback(event.payload),
  );
}

/** Payload emitted after a pull when submodule references have changed. */
export interface SubmoduleRefsChangedEvent {
  tab_id: string;
  changed_submodules: string[];
}

/** Listen for submodule reference changes after a pull operation. */
export function onSubmoduleRefsChanged(
  callback: (payload: SubmoduleRefsChangedEvent) => void,
): Promise<UnlistenFn> {
  return listen<SubmoduleRefsChangedEvent>('submodule-refs-changed', (event) =>
    callback(event.payload),
  );
}
