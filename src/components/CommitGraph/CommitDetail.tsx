import { useTranslation } from 'react-i18next';
import type { CommitInfo, CommitDetail as CommitDetailType, FileStatus } from '../../ipc/types';

interface Props {
  detail: CommitDetailType | null;
  loading: boolean;
}

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

function statusIcon(status: FileStatus['status']): string {
  switch (status) {
    case 'Modified': return 'M';
    case 'Staged': return 'S';
    case 'Untracked': return '?';
    case 'Deleted': return 'D';
    case 'Renamed': return 'R';
    case 'Conflict': return 'C';
  }
}

export function CommitDetailPanel({ detail, loading }: Props) {
  const { t } = useTranslation();

  if (loading) {
    return <div className="commit-detail-loading">{t('common.loading')}</div>;
  }

  if (!detail) {
    return null;
  }

  const { commit, files, stats } = detail;

  return (
    <div className="commit-detail" role="region" aria-label="Commit details">
      <div className="commit-detail-header">
        <div className="commit-detail-message">{commit.message}</div>
        <div className="commit-detail-meta">
          <span className="commit-detail-author">
            {commit.author.name} &lt;{commit.author.email}&gt;
          </span>
          <span className="commit-detail-date">{formatDate(commit.author.timestamp)}</span>
          <span className="commit-detail-sha" title={commit.id}>
            {commit.short_id}
          </span>
          {commit.is_cherry_picked && (
            <span className="commit-detail-cherry-pick" title="Cherry-picked">🍒</span>
          )}
        </div>
        {/* Submodule reference changes */}
        {files.some((f) => f.path.includes('submodule') || f.status === 'Modified') && (
          <div className="commit-detail-submodule-hint">
            {/* Submodule changes are identified by path patterns in the file list */}
          </div>
        )}
      </div>

      <div className="commit-detail-stats">
        <span>{stats.files_changed} files changed</span>
        <span className="commit-detail-insertions">+{stats.insertions}</span>
        <span className="commit-detail-deletions">-{stats.deletions}</span>
      </div>

      <ul className="commit-detail-files" role="list">
        {files.map((file) => (
          <li key={file.path} className="commit-detail-file" role="listitem">
            <span className={`commit-detail-file-status commit-detail-file-status--${file.status.toLowerCase()}`}>
              {statusIcon(file)}
            </span>
            <span className="commit-detail-file-path">{file.path}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
