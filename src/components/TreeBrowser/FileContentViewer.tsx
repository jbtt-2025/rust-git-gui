import { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { TabId } from '../../ipc/types';
import { gitApi } from '../../ipc/client';

export interface FileContentViewerProps {
  tabId: TabId;
  commitId: string;
  filePath: string | null;
}

export function FileContentViewer({ tabId, commitId, filePath }: FileContentViewerProps) {
  const { t } = useTranslation();
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!filePath) {
      setContent(null);
      return;
    }

    let cancelled = false;
    setLoading(true);

    gitApi.getFileContent(tabId, commitId, filePath).then(
      (text) => {
        if (!cancelled) {
          setContent(text);
          setLoading(false);
        }
      },
      () => {
        if (!cancelled) {
          setContent(null);
          setLoading(false);
        }
      },
    );

    return () => {
      cancelled = true;
    };
  }, [tabId, commitId, filePath]);

  const lines = useMemo(() => {
    if (content == null) return [];
    return content.split('\n');
  }, [content]);

  if (!filePath) {
    return (
      <div className="file-content-viewer">
        <div className="file-content-viewer-empty">
          {t('treeBrowser.noContent')}
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="file-content-viewer">
        <div className="file-content-viewer-loading">
          {t('treeBrowser.loading')}
        </div>
      </div>
    );
  }

  return (
    <div className="file-content-viewer">
      <div className="file-content-viewer-path">{filePath}</div>
      <pre className="file-content-viewer-body">
        <table className="file-content-table">
          <tbody>
            {lines.map((line, idx) => (
              <tr key={idx}>
                <td className="file-content-line-no">{idx + 1}</td>
                <td className="file-content-line-text">{line}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </pre>
    </div>
  );
}
