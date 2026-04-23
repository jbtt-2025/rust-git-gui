import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import * as ContextMenu from '@radix-ui/react-context-menu';
import type { BranchInfo } from '../../ipc/types';
import { SidebarSection } from './SidebarSection';
import { BranchActions } from './BranchActions';
import { groupBranchesByRemote, stripRemotePrefix } from './sidebarUtils';

interface Props {
  branches: BranchInfo[];
  soloedBranches: Set<string>;
  hiddenBranches: Set<string>;
  onSolo: (branch: string) => void;
  onHide: (branch: string) => void;
  onFetch: (remote: string) => void;
}

export function RemotesSection({
  branches,
  soloedBranches,
  hiddenBranches,
  onSolo,
  onHide,
  onFetch,
}: Props) {
  const { t } = useTranslation();
  const grouped = useMemo(() => groupBranchesByRemote(branches), [branches]);

  return (
    <SidebarSection id="remotes" title={t('sidebar.remotes')} count={branches.length}>
      {Array.from(grouped.entries()).map(([remote, remoteBranches]) => (
        <div key={remote} className="sidebar-remote-group">
          <ContextMenu.Root>
            <ContextMenu.Trigger asChild>
              <div className="sidebar-remote-header">
                <span className="sidebar-item-icon">🌐</span>
                <span className="sidebar-item-label">{remote}</span>
              </div>
            </ContextMenu.Trigger>
            <ContextMenu.Portal>
              <ContextMenu.Content className="context-menu">
                <ContextMenu.Item
                  className="context-menu-item"
                  onSelect={() => onFetch(remote)}
                >
                  {t('remote.fetch')} {remote}
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Portal>
          </ContextMenu.Root>

          <ul className="sidebar-list sidebar-list--nested" role="list">
            {remoteBranches.map((branch) => (
              <li key={branch.name} className="sidebar-item" role="listitem">
                <span className="sidebar-item-label" title={branch.name}>
                  {stripRemotePrefix(branch.name)}
                </span>
                <BranchActions
                  branchName={branch.name}
                  isSoloed={soloedBranches.has(branch.name)}
                  isHidden={hiddenBranches.has(branch.name)}
                  onSolo={onSolo}
                  onHide={onHide}
                />
              </li>
            ))}
          </ul>
        </div>
      ))}
    </SidebarSection>
  );
}
