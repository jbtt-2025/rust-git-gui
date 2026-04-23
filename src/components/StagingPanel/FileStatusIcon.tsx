import type { FileStatusType } from '../../ipc/types';
import { getFileStatusIcon } from './statusIconMap';

export interface FileStatusIconProps {
  status: FileStatusType;
}

export function FileStatusIcon({ status }: FileStatusIconProps) {
  const { icon, color, label } = getFileStatusIcon(status);
  return (
    <span className={`${color} text-sm`} title={label} aria-label={label}>
      {icon}
    </span>
  );
}
