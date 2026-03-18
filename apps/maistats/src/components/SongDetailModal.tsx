import { useEffect, useState } from 'react';

import { formatApiErrorMessage } from '../api';
import { useI18n } from '../app/i18n';
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
  const { locale, t } = useI18n();
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
      setRefreshError(t('songDetail.refreshUnavailable'));
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
      setRefreshError(formatApiErrorMessage(error, t));
    } finally {
      setIsRefreshing(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section className="modal-card panel" onClick={(event) => event.stopPropagation()}>
        <h2>{t('songDetail.title')}</h2>
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
                {isRefreshing ? t('songDetail.refreshing') : t('songDetail.refresh')}
              </button>
              <button type="button" className="modal-close-button" onClick={onClose}>
                {t('common.close')}
              </button>
            </div>
          </div>
          {refreshError ? <p className="error-inline">{refreshError}</p> : null}
          {selectedDetailRows.length === 0 ? (
            <p className="muted">{t('songDetail.empty')}</p>
          ) : null}
          {selectedDetailRows.length > 0 ? (
            <div className="table-wrap">
              <table className="detail-table compact-table">
                <thead>
                  <tr>
                    <th>{t('common.chart')}</th>
                    <th>{t('common.levelShort')}</th>
                    <th>{t('common.achievementShort')}</th>
                    <th>{t('common.rank')}</th>
                    <th>{t('common.fc')}</th>
                    <th>{t('common.sync')}</th>
                    <th>{t('common.dx')}</th>
                    <th>{t('common.lastPlayed')}</th>
                    <th>{t('common.playCount')}</th>
                    <th>{t('common.version')}</th>
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
                        {formatNumber(row.dxScore, locale)} / {formatNumber(row.dxScoreMax, locale)}
                      </td>
                      <td>{row.lastPlayedAtLabel ?? '-'}</td>
                      <td>{formatNumber(row.playCount, locale)}</td>
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
