import { useState, useEffect, useCallback } from 'react';
import { onSubmoduleRefsChanged } from '../ipc/events';
import { gitApi } from '../ipc/client';

/**
 * Hook that listens for submodule reference changes after pull operations
 * and provides state/actions for the update banner.
 */
export function useSubmoduleUpdates(tabId: string | null) {
  const [showBanner, setShowBanner] = useState(false);
  const [changedSubmodules, setChangedSubmodules] = useState<string[]>([]);

  useEffect(() => {
    if (!tabId) return;

    const unlisten = onSubmoduleRefsChanged((payload) => {
      if (payload.tab_id === tabId && payload.changed_submodules.length > 0) {
        setChangedSubmodules(payload.changed_submodules);
        setShowBanner(true);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [tabId]);

  const updateAll = useCallback(async () => {
    if (!tabId || changedSubmodules.length === 0) return;
    try {
      for (const subPath of changedSubmodules) {
        await gitApi.updateSubmodule(tabId, subPath, true);
      }
    } finally {
      setShowBanner(false);
      setChangedSubmodules([]);
    }
  }, [tabId, changedSubmodules]);

  const dismiss = useCallback(() => {
    setShowBanner(false);
    setChangedSubmodules([]);
  }, []);

  return { showBanner, changedSubmodules, updateAll, dismiss };
}
