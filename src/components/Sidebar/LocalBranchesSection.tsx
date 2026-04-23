import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { BranchInfo } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';
import { BranchActions } from './BranchActions';

interface Props {
  branches: BranchInfo[];
  tabId: string;
  soloedBranches: Set<string>;
  hiddenBranches: Set<string>;
  onSolo: (branch: string) => void;
  onHide: (branch: string) => void;
  onCreateBranch: () => void;
  onDeleteBranch: (name: string) => void;
  onCheckout: (name: string) => void;
}

export function LocalBranchesSection({
  branches,
  tabId: _tabId,
  soloedBranches,
  hiddenBranches,
  onSolo,
  onHide,
  onCreateBranch,
  onDeleteBranch,
  onCheckout,
}: Props) {
  const { t } = useTranslation();

  return (
    <SidebarSection id="local" title={t('sidebar.local')} count={branches.length}>
      <ul className="sidebar-list" role="list">
        {branches.map((branch) => (
          <ContextMenu.Root key={branch.name}>
            <ContextMenu.Trigger asChild>
              <li
                className="sidebar-item"
                data-active={branch.is_head || undefined}
                role="listitem"
              >
                <span className="sidebar-item-icon">
                  {branch.is_head ? '●' : '○'}
                </span>
                <span className="sidebar-item-label" title={branch.name}>
                  {branch.name}
                </span>
                <BranchActions
                  branchName={branch.name}
                  isSoloed={soloedBranches.has(branch.name)}
                  isHidden={hiddenBranches.has(branch.name)}
                  onSolo={onSolo}
                  onHide={onHide}
                />
              </li>
            </ContextMenu.Trigger>

            <ContextMenu.Portal>
              <ContextMenu.Content className="context-menu">
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onCheckout(branch.name)}
                >
                  {t('branch.checkout')}
                </ContextMenu.Item>
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={onCreateBranch}
                >
                  {t('branch.create')}
                </ContextMenu.Item>
                <ContextMenu.Separator className="context-menu-separator" />
                <ContextMenu.Item
                  className="context-menu-item context-menu-item--danger"
                  onSelect={() => onDeleteBranch(branch.name)}
                >
                  {t('branch.delete')}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>
        ))}
      </ul>
    </SidebarSection>
  );
}
