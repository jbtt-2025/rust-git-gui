import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { StashEntry } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';

interface Props {
  stashes: StashEntry[];
  onApply: (index: number) => void;
  onPop: (index: number) => void;
  onDrop: (index: number) => void;
  onSelect: (index: number) => void;
}

export function StashesSection({ stashes, onApply, onPop, onDrop, onSelect }: Props) {
  const { t } = useTranslation();

  return (
    <SidebarSection id="stashes" title={t('sidebar.stashes')} count={stashes.length}>
      <ul className="sidebar-list" role="list">
        {stashes.map((stash) => (
          <ContextMenu.Root key={stash.index}>
            <ContextMenu.Trigger asChild>
              <li
                className="sidebar-item"
                role="listitem"
                onClick={() => onSelect(stash.index)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') onSelect(stash.index);
                }}
                tabIndex={0}
              >
                <span className="sidebar-item-icon">📦</span>
                <span className="sidebar-item-label" title={stash.message || `stash@{${stash.index}}`}>
                  {stash.message || `stash@{${stash.index}}`}
                </span>
              </li>
            </ContextMenu.Trigger>

            <ContextMenu.Portal>
              <ContextMenu.Content className="context-menu">
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onApply(stash.index)}
                >
                  {t('stash.apply')}
                </ContextMenu.Item>
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onPop(stash.index)}
                >
                  {t('stash.pop')}
                </ContextMenu.Item>
                <ContextMenu.Separator className="context-menu-separator" />
                <ContextMenu.Item
                  className="context-menu-item context-menu-item--danger"
                  onSelect={() => onDrop(stash.index)}
                >
                  {t('stash.drop')}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
        ))}
      </ul>
    </SidebarSection>
  );
}
