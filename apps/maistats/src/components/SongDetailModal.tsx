import { useEffect, useState } from 'react';

import { ChartTypeLabel } from './ChartTypeLabel';
import { Jacket } from './Jacket';
import { getDifficultyToneClass } from './DifficultyLabel';
import type { SongAliases, SongDetailRow } from '../types';
import type { SongDetailTarget } from './TableActionCells';
import {
  aliasValues,
  formatVersionLabel,
  formatNumber,
  formatPercent,
} from '../app/utils';

interface SongDetailModalProps {
  selectedDetailTitle: string | null;
  selectedDetailGenre: string | null;
  selectedDetailArtist: string | null;
  selectedDetailAliases: SongAliases | null;
  selectedDetailRows: SongDetailRow[];
  songInfoUrl: string;
  recordCollectorUrl: string;
  onRefreshSongScores: (target: SongDetailTarget) => Promise<void>;
  onClose: () => void;
}

export function SongDetailModal({
  selectedDetailTitle,
  selectedDetailGenre,
  selectedDetailArtist,
  selectedDetailAliases,
  selectedDetailRows,
  songInfoUrl,
  recordCollectorUrl,
  onRefreshSongScores,
  onClose,
}: SongDetailModalProps) {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);

  const imageName = selectedDetailRows[0]?.imageName ?? null;
  const englishAliases = aliasValues(selectedDetailAliases, 'en');
  const koreanAliases = aliasValues(selectedDetailAliases, 'ko');
  const canRefresh =
    recordCollectorUrl.trim().length > 0 &&
    selectedDetailGenre !== null &&
    selectedDetailArtist !== null;

  useEffect(() => {
    setIsRefreshing(false);
    setRefreshError(null);
  }, [selectedDetailArtist, selectedDetailGenre, selectedDetailTitle]);

  if (selectedDetailTitle === null) {
    return null;
  }

  const renderInternalLevel = (row: SongDetailRow) => {
    if (row.internalLevel === null) {
      return '-';
    }

    const [whole, fraction = '0'] = row.internalLevel.toFixed(1).split('.');
    if (!row.isInternalLevelEstimated) {
      return `${whole}.${fraction}`;
    }

    return (
      <span className={`estimated-level ${getDifficultyToneClass(row.difficulty)}`}>
        {whole}
        <span className="estimated-level-fraction">.{fraction}</span>
      </span>
    );
  };

  const renderLevelCell = (row: SongDetailRow) => {
    if (row.internalLevel === null) {
      return '-';
    }

    if (row.isInternalLevelEstimated) {
      return (
        <span className={`level-badge ${getDifficultyToneClass(row.difficulty)}`}>
          {renderInternalLevel(row)}
        </span>
      );
    }

    return (
      <span className={`level-badge ${getDifficultyToneClass(row.difficulty)}`}>
        {row.internalLevel.toFixed(1)}
      </span>
    );
  };

  const handleRefreshClick = async () => {
    if (selectedDetailGenre === null || selectedDetailArtist === null) {
      setRefreshError('곡 식별 정보가 부족해서 새로고침할 수 없습니다.');
      return;
    }

    setIsRefreshing(true);
    setRefreshError(null);
    try {
      await onRefreshSongScores({
        title: selectedDetailTitle,
        genre: selectedDetailGenre,
        artist: selectedDetailArtist,
      });
    } catch (error) {
      setRefreshError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsRefreshing(false);
    }
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
              <div>
                <strong>{selectedDetailTitle}</strong>
                {selectedDetailGenre || selectedDetailArtist ? (
                  <div className="muted">
                    {[selectedDetailGenre, selectedDetailArtist]
                      .filter((value): value is string => Boolean(value))
                      .join(' / ')}
                  </div>
                ) : null}
                {englishAliases.length > 0 ? (
                  <div className="muted detail-aliases">EN: {englishAliases.join(', ')}</div>
                ) : null}
                {koreanAliases.length > 0 ? (
                  <div className="muted detail-aliases">KO: {koreanAliases.join(', ')}</div>
                ) : null}
              </div>
            </div>
            <div className="modal-header-actions">
              <button
                type="button"
                className="modal-refresh-button"
                onClick={handleRefreshClick}
                disabled={!canRefresh || isRefreshing}
              >
                {isRefreshing ? '갱신 중...' : 'Score 갱신'}
              </button>
              <button type="button" className="modal-close-button" onClick={onClose}>
                닫기
              </button>
            </div>
          </div>
          {refreshError ? <p className="error-inline">{refreshError}</p> : null}
          {selectedDetailRows.length === 0 ? (
            <p className="muted">조회 가능한 상세 데이터가 없습니다.</p>
          ) : null}
          {selectedDetailRows.length > 0 ? (
            <div className="table-wrap">
              <table className="detail-table compact-table">
                <thead>
                  <tr>
                    <th>Chart</th>
                    <th>Lv</th>
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
                      <td>
                        <ChartTypeLabel chartType={row.chartType} />
                      </td>
                      <td>{renderLevelCell(row)}</td>
                      <td>{formatPercent(row.achievementPercent)}</td>
                      <td>{row.rank ?? '-'}</td>
                      <td>{row.fc ?? '-'}</td>
                      <td>{row.sync ?? '-'}</td>
                      <td>
                        {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
                      </td>
                      <td>{row.lastPlayedAtLabel ?? '-'}</td>
                      <td>{formatNumber(row.playCount)}</td>
                      <td>{formatVersionLabel(row.version)}</td>
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
