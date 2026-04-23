// IPC client barrel export
export * from './types';
export { gitApi } from './client';
export { onFileChanged, onProgress, onSubmoduleRefsChanged } from './events';
export type { SubmoduleRefsChangedEvent } from './events';
