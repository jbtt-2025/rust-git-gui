import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { WorktreeInfo } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';

export interface WorktreeSectionProps {
  worktrees: WorktreeInfo[];
  onSelect: (worktree: WorktreeInfo) => void;
  onDelete: (name: string) => void;
}

export function WorktreeSection({ worktrees, onSelect, onDelete }: WorktreeSectionProps) {
  const { t } = useTranslation();

  return (
    <SidebarSection id="worktrees" title={t('sidebar.worktrees')} count={worktrees.length}>
      <ul className="sidebar-list" role="list">
        {worktrees.map((wt) => (
          <ContextMenu.Root key={wt.name}>
            <ContextMenu.Trigger asChild>
              <li
                className="sidebar-item"
                role="listitem"
                data-main={wt.is_main || undefined}
                onClick={() => onSelect(wt)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') onSelect(wt);
                }}
                tabIndex={0}
                title={wt.path}
              >
                <span className="sidebar-item-icon">{wt.is_main ? '🏠' : '📂'}</span>
                <span className="sidebar-item-label">
                  {wt.name}
                  {wt.branch && (
                    <span className="sidebar-item-branch" data-testid={`wt-branch-${wt.name}`}>
                      {' '}
                      [{wt.branch}]
                    </span>
                  )}
                  {wt.is_main && (
                    <span className="sidebar-item-badge" data-testid={`wt-main-${wt.name}`}>
                      {' '}
                      {t('worktree.main')}
                    </span>
                  )}
                </span>
              </li>
            </ContextMenu.Trigger>

            <ContextMenu.Portal>
              <ContextMenu.Content className="context-menu">
                <ContextMenu.Item
                  className="context-menu-item context-menu-item--danger"
                  onSelect={() => onDelete(wt.name)}
                  disabled={wt.is_main}
                >
                  {t('worktree.delete')}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
        ))}
      </ul>
    </SidebarSection>
  );
}
