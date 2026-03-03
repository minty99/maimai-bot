import { Jacket } from './Jacket';
import type { SongDetailRow } from '../types';
import {
  formatDifficultyShort,
  formatNumber,
  formatPercent,
} from '../app/utils';

interface SongDetailModalProps {
  selectedDetailTitle: string | null;
  selectedDetailRows: SongDetailRow[];
  songInfoUrl: string;
  onClose: () => void;
}

export function SongDetailModal({
  selectedDetailTitle,
  selectedDetailRows,
  songInfoUrl,
  onClose,
}: SongDetailModalProps) {
  if (!selectedDetailTitle) {
    return null;
  }

  const imageName = selectedDetailRows[0]?.imageName ?? null;

  const renderInternalLevel = (row: SongDetailRow) => {
    if (row.internalLevel === null) {
      return '-';
    }

    const [whole, fraction = '0'] = row.internalLevel.toFixed(1).split('.');
    if (!row.isInternalLevelEstimated) {
      return `${whole}.${fraction}`;
    }

    return (
      <span className="estimated-level">
        {whole}
        <span className="estimated-level-fraction">.{fraction}</span>
      </span>
    );
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section className="modal-card panel" onClick={(event) => event.stopPropagation()}>
        <h2>Song Detail</h2>
        <div className="detail-content">
          <div className="detail-header">
            <div className="detail-song-summary">
              <Jacket
                songInfoUrl={songInfoUrl}
                imageName={imageName}
                title={selectedDetailTitle}
                className="detail-jacket"
              />
              <strong>{selectedDetailTitle}</strong>
            </div>
            <button type="button" onClick={onClose}>
              닫기
            </button>
          </div>
          {selectedDetailRows.length === 0 ? (
            <p className="muted">조회 가능한 상세 데이터가 없습니다.</p>
          ) : null}
          {selectedDetailRows.length > 0 ? (
            <div className="table-wrap">
              <table className="detail-table compact-table">
                <thead>
                  <tr>
                    <th>Chart</th>
                    <th>Diff</th>
                    <th>Lv</th>
                    <th>IntLv</th>
                    <th>User Lv</th>
                    <th>Achv</th>
                    <th>Rank</th>
                    <th>FC</th>
                    <th>Sync</th>
                    <th>DX</th>
                    <th>Last Played</th>
                    <th>Play Count</th>
                    <th>Version</th>
                  </tr>
                </thead>
                <tbody>
                  {selectedDetailRows.map((row) => (
                    <tr key={row.key}>
                      <td>{row.chartType}</td>
                      <td>{formatDifficultyShort(row.difficulty)}</td>
                      <td>{row.level ?? '-'}</td>
                      <td>{renderInternalLevel(row)}</td>
                      <td>{row.userLevel ?? '-'}</td>
                      <td>{formatPercent(row.achievementPercent)}</td>
                      <td>{row.rank ?? '-'}</td>
                      <td>{row.fc ?? '-'}</td>
                      <td>{row.sync ?? '-'}</td>
                      <td>
                        {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
                      </td>
                      <td>{row.lastPlayedAtLabel ?? '-'}</td>
                      <td>{formatNumber(row.playCount)}</td>
                      <td>{row.version ?? '-'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : null}
        </div>
      </section>
    </div>
  );
}
