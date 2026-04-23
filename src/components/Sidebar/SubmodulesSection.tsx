import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { SubmoduleInfo } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';

interface Props {
  submodules: SubmoduleInfo[];
  onInit: (path: string) => void;
  onUpdate: (path: string) => void;
  onDeinit: (path: string) => void;
  onOpen: (path: string) => void;
  onCopyPath: (path: string) => void;
  onChangeUrl: (path: string) => void;
}

function statusBadge(status: SubmoduleInfo['status']): string {
  switch (status) {
    case 'Uninitialized': return '⬜';
    case 'Initialized': return '✅';
    case 'Modified': return '🔶';
    case 'DetachedHead': return '⚠️';
  }
}

export function SubmodulesSection({
  submodules,
  onInit,
  onUpdate,
  onDeinit,
  onOpen,
  onCopyPath,
  onChangeUrl,
}: Props) {
  const { t } = useTranslation();

  return (
    <SidebarSection id="submodules" title={t('sidebar.submodules')} count={submodules.length}>
      <ul className="sidebar-list" role="list">
        {submodules.map((sub) => (
          <ContextMenu.Root key={sub.path}>
            <ContextMenu.Trigger asChild>
              <li className="sidebar-item" role="listitem">
                <span className="sidebar-item-icon">{statusBadge(sub.status)}</span>
                <span className="sidebar-item-label" title={sub.path}>
                  {sub.name}
                </span>
              </li>
            </ContextMenu.Trigger>

            <ContextMenu.Portal>
              <ContextMenu.Content className="context-menu">
                {sub.status === 'Uninitialized' && (
                  <ContextMenu.Item
                    className="context-menu-item"
                    onSelect={() => onInit(sub.path)}
                  >
                    {t('submodule.init')}
                  </ContextMenu.Item>
                )}
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onUpdate(sub.path)}
                >
                  {t('submodule.update')}
                </ContextMenu.Item>
                {sub.status !== 'Uninitialized' && (
                  <ContextMenu.Item
                    className="context-menu-item"
                    onSelect={() => onDeinit(sub.path)}
                  >
                    {t('submodule.deinit')}
                  </ContextMenu.Item>
                )}
                <ContextMenu.Separator className="context-menu-separator" />
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onOpen(sub.path)}
                >
                  {t('submodule.open')}
                </ContextMenu.Item>
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onCopyPath(sub.path)}
                >
                  {t('submodule.copyPath')}
                </ContextMenu.Item>
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onChangeUrl(sub.path)}
                >
                  {t('submodule.changeUrl')}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
        ))}
      </ul>
    </SidebarSection>
  );
}
