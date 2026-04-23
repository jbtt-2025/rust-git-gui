import type { FileStatusType } from '../../ipc/types';

export interface FileStatusIconInfo {
  icon: string;
  color: string;
  label: string;
}

/**
 * Pure mapping function: each FileStatusType maps to a unique icon/color/label.
 * This function is property-tested in task 14.4 (Property 10).
 */
export function getFileStatusIcon(status: FileStatusType): FileStatusIconInfo {
  switch (status) {
    case 'Untracked':
      return { icon: '◆', color: 'text-green-400', label: 'Untracked' };
    case 'Modified':
      return { icon: '●', color: 'text-yellow-400', label: 'Modified' };
    case 'Staged':
      return { icon: '✔', color: 'text-blue-400', label: 'Staged' };
    case 'Conflict':
      return { icon: '⚠', color: 'text-red-400', label: 'Conflict' };
    case 'Deleted':
      return { icon: '✖', color: 'text-red-500', label: 'Deleted' };
    case 'Renamed':
      return { icon: '➜', color: 'text-purple-400', label: 'Renamed' };
  }
}
