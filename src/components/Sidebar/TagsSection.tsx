import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { TagInfo } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';

interface Props {
  tags: TagInfo[];
  onCheckoutTag: (name: string) => void;
  onDeleteTag: (name: string) => void;
  onPushTag: (name: string) => void;
  onCopyTagName: (name: string) => void;
}

export function TagsSection({
  tags,
  onCheckoutTag,
  onDeleteTag,
  onPushTag,
  onCopyTagName,
}: Props) {
  const { t } = useTranslation();

  return (
    <SidebarSection id="tags" title={t('sidebar.tags')} count={tags.length}>
      <ul className="sidebar-list" role="list">
        {tags.map((tag) => (
          <ContextMenu.Root key={tag.name}>
            <ContextMenu.Trigger asChild>
              <li className="sidebar-item" role="listitem">
                <span className="sidebar-item-icon">🏷️</span>
                <span className="sidebar-item-label" title={tag.name}>
                  {tag.name}
                </span>
              </li>
            </ContextMenu.Trigger>

            <ContextMenu.Portal>
              <ContextMenu.Content className="context-menu">
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onCheckoutTag(tag.name)}
                >
                  {t('branch.checkout')}
                </ContextMenu.Item>
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onPushTag(tag.name)}
                >
                  {t('tag.pushToRemote')}
                </ContextMenu.Item>
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onCopyTagName(tag.name)}
                >
                  {t('tag.copyName')}
                </ContextMenu.Item>
                <ContextMenu.Separator className="context-menu-separator" />
                <ContextMenu.Item
                  className="context-menu-item context-menu-item--danger"
                  onSelect={() => onDeleteTag(tag.name)}
                >
                  {t('tag.delete')}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
        ))}
      </ul>
    </SidebarSection>
  );
}
