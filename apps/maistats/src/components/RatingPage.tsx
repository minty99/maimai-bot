import type { KeyboardEvent, ReactNode } from 'react';

import { useI18n } from '../app/i18n';
import { toIntegerRating } from '../derive';
import { formatNumber, formatPercent, formatVersionLabel } from '../app/utils';
import type { ScoreRow } from '../types';
import { ChartTypeLabel } from './ChartTypeLabel';
import { DifficultyLabel, getDifficultyToneClass } from './DifficultyLabel';
import { Jacket } from './Jacket';
import { LevelCell } from './LevelCell';
import type { SongDetailTarget } from './TableActionCells';

interface RatingPageProps {
  sidebarTopContent?: ReactNode;
  songInfoUrl: string;
  ratingTotal: number;
  newRatingTotal: number;
  oldRatingTotal: number;
  newRows: ScoreRow[];
  oldRows: ScoreRow[];
  onOpenSongDetail: (target: SongDetailTarget) => void;
}

function formatRatingAvg(total: number, count: number): string {
  if (count === 0) return '-';
  return (total / count).toFixed(2);
}

function formatRatingProjection(total: number, count: number, locale: string): string {
  if (count === 0) return '-';
  const avg = total / count;
  return Math.round(avg * 50).toLocaleString(locale);
}

function handleCardKeyDown(event: KeyboardEvent<HTMLElement>, onOpenSongDetail: () => void) {
  if (event.key !== 'Enter' && event.key !== ' ') {
    return;
  }

  event.preventDefault();
  onOpenSongDetail();
}

function RatingCardSection({
  title,
  summary,
  rows,
  songInfoUrl,
  onOpenSongDetail,
}: {
  title: string;
  summary: string;
  rows: ScoreRow[];
  songInfoUrl: string;
  onOpenSongDetail: (target: SongDetailTarget) => void;
}) {
  const { locale, t } = useI18n();
  return (
    <section className="panel rating-section-panel">
      <div className="panel-heading">
        <div>
          <h2>{title}</h2>
        </div>
        <span className="panel-count">{summary}</span>
      </div>
      <div className="rating-card-grid">
        {rows.map((row, index) => {
          const handleOpenDetail = () => onOpenSongDetail(row);

          return (
            <article
              key={row.key}
              className={`rating-song-card ${getDifficultyToneClass(row.difficulty)}`}
              role="button"
              tabIndex={0}
              aria-label={t('rating.openSongDetail', { title: row.title })}
              onClick={handleOpenDetail}
              onKeyDown={(event) => handleCardKeyDown(event, handleOpenDetail)}
            >
              <div className={`rating-song-stage ${getDifficultyToneClass(row.difficulty)}`}>
                <div className="rating-song-jacket-wrap">
                  <Jacket
                    songInfoUrl={songInfoUrl}
                    imageName={row.imageName}
                    title={row.title}
                    className="rating-song-jacket"
                  />
                </div>
                <div className="rating-song-stage-gradient" />
                <div className="rating-song-stage-topline">
                  <span>#{index + 1}</span>
                  <span>{formatVersionLabel(row.version)}</span>
                </div>
                <div className="rating-song-stage-badges">
                  <ChartTypeLabel chartType={row.chartType} />
                  <DifficultyLabel difficulty={row.difficulty} short className="rating-difficulty-chip" />
                </div>
                <div className="rating-song-rating-chip">
                  <strong>{formatNumber(toIntegerRating(row.rating), locale)}</strong>
                </div>
              </div>
              <div className="rating-song-info">
                <h3>{row.title}</h3>
                <div className="rating-song-level-row">
                  <span>{row.level ? `Lv ${row.level}` : 'Lv -'}</span>
                  <LevelCell
                    internalLevel={row.internalLevel}
                    isInternalLevelEstimated={row.isInternalLevelEstimated}
                    difficulty={row.difficulty}
                  />
                </div>
                <div className="rating-song-stat-grid">
                  <div className="rating-song-stat">
                    <strong>{formatPercent(row.achievementPercent)}</strong>
                  </div>
                  <div className="rating-song-stat">
                    <strong>{row.rank ?? '-'}</strong>
                  </div>
                  <div className="rating-song-stat">
                    <strong>{row.fc ?? '-'}</strong>
                  </div>
                </div>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

export function RatingPage({
  sidebarTopContent,
  songInfoUrl,
  ratingTotal,
  newRatingTotal,
  oldRatingTotal,
  newRows,
  oldRows,
  onOpenSongDetail,
}: RatingPageProps) {
  const { locale, t } = useI18n();
  const newSummary = `avg ${formatRatingAvg(newRatingTotal, newRows.length)} (~${formatRatingProjection(newRatingTotal, newRows.length, locale)})`;
  const oldSummary = `avg ${formatRatingAvg(oldRatingTotal, oldRows.length)} (~${formatRatingProjection(oldRatingTotal, oldRows.length, locale)})`;

  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">
        {sidebarTopContent}
        <section className="panel filter-panel">
          <div className="panel-heading compact">
            <div>
              <h2>RATING</h2>
            </div>
          </div>
          <div className="rating-stat-grid">
            <div className="rating-stat-card">
              <span>{t('rating.current')}</span>
              <strong>{formatNumber(ratingTotal, locale)}</strong>
              <small className="rating-stat-sub">
                {t('rating.avg', { value: formatRatingAvg(ratingTotal, newRows.length + oldRows.length) })}
              </small>
            </div>
          </div>
        </section>
      </aside>

      <div className="table-column rating-table-column">
        <RatingCardSection
          title="NEW"
          summary={newSummary}
          rows={newRows}
          songInfoUrl={songInfoUrl}
          onOpenSongDetail={onOpenSongDetail}
        />
        <RatingCardSection
          title="OLD"
          summary={oldSummary}
          rows={oldRows}
          songInfoUrl={songInfoUrl}
          onOpenSongDetail={onOpenSongDetail}
        />
      </div>
    </div>
  );
}
