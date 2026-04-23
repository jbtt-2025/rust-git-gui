import { type ReactNode, useCallback } from 'react';
import { useUiStore } from '../../stores';

interface SidebarSectionProps {
  id: string;
  title: string;
  count: number;
  children: ReactNode;
}

/**
 * A collapsible sidebar section with a title and item count badge.
 * Collapse state is persisted in the UI store.
 */
export function SidebarSection({ id, title, count, children }: SidebarSectionProps) {
  const collapsed = useUiStore((s) => s.sidebarCollapsed[id] ?? false);
  const toggle = useUiStore((s) => s.toggleSidebarSection);

  const handleToggle = useCallback(() => toggle(id), [toggle, id]);

  return (
    <div className="sidebar-section" data-section={id}>
      <button
        type="button"
        className="sidebar-section-header"
        onClick={handleToggle}
        aria-expanded={!collapsed}
        aria-controls={`sidebar-section-${id}`}
      >
        <span className="sidebar-chevron" data-collapsed={collapsed}>
          ▶
        </span>
        <span className="sidebar-section-title">{title}</span>
        <span className="sidebar-section-count" data-testid={`count-${id}`}>
          {count}
        </span>
      </button>

      {!collapsed && (
        <div
          id={`sidebar-section-${id}`}
          className="sidebar-section-content"
          role="group"
          aria-label={title}
        >
          {children}
        </div>
      )}
    </div>
  );
}
