import type { ReactNode } from 'react';

import { toDateLabel } from '../derive';
import { formatNumber, formatVersionLabel } from '../app/utils';
import type { ScoreRow } from '../types';
import { ChartTypeLabel } from './ChartTypeLabel';
import { Jacket } from './Jacket';
import { LevelCell } from './LevelCell';
import type { SongDetailTarget } from './TableActionCells';
import { AchievementHistoryButton, SongTitleButton } from './TableActionCells';

interface RatingPageProps {
  sidebarTopContent?: ReactNode;
  songInfoUrl: string;
  ratingTotal: number;
  newRatingTotal: number;
  oldRatingTotal: number;
  newRows: ScoreRow[];
  oldRows: ScoreRow[];
  onOpenSongDetail: (target: SongDetailTarget) => void;
  onOpenHistory: (row: ScoreRow) => void;
}

function formatRatingAvg(total: number, count: number): string {
  if (count === 0) return '-';
  return (total / count).toFixed(2);
}

function formatRatingProjection(total: number, count: number): string {
  if (count === 0) return '-';
  const avg = total / count;
  return Math.round(avg * 50).toLocaleString();
}

function RatingTable({
  title,
  description,
  rows,
  songInfoUrl,
  onOpenSongDetail,
  onOpenHistory,
}: {
  title: string;
  description: string;
  rows: ScoreRow[];
  songInfoUrl: string;
  onOpenSongDetail: (target: SongDetailTarget) => void;
  onOpenHistory: (row: ScoreRow) => void;
}) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <div>
          <h2>{title}</h2>
          <p>{description}</p>
        </div>
        <span className="panel-count">{rows.length.toLocaleString()}곡</span>
      </div>
      <div className="table-wrap">
        <table className="score-table compact-table">
          <thead>
            <tr>
              <th className="jacket-col">Jacket</th>
              <th className="title-col">Title</th>
              <th className="chart-col">Chart</th>
              <th className="level-col">Lv</th>
              <th className="achievement-col">Achv</th>
              <th className="rating-col">Rating</th>
              <th className="rank-col">Rank</th>
              <th className="fc-col">FC</th>
              <th className="sync-col">Sync</th>
              <th className="dx-col">DX</th>
              <th className="last-played-col">Last Played</th>
              <th className="play-count-col">Play count</th>
              <th className="version-col">Version</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.key}>
                <td className="jacket-col">
                  <Jacket songInfoUrl={songInfoUrl} imageName={row.imageName} title={row.title} />
                </td>
                <td className="title-col">
                  <div className="title-cell">
                    <SongTitleButton
                      target={row}
                      title={row.title}
                      onOpenSongDetail={onOpenSongDetail}
                    />
                  </div>
                </td>
                <td className="chart-col">
                  <ChartTypeLabel chartType={row.chartType} />
                </td>
                <td className="level-col">
                  <LevelCell
                    internalLevel={row.internalLevel}
                    isInternalLevelEstimated={row.isInternalLevelEstimated}
                    difficulty={row.difficulty}
                  />
                </td>
                <td className="achievement-col">
                  <AchievementHistoryButton
                    achievementPercent={row.achievementPercent}
                    onOpenHistory={() => onOpenHistory(row)}
                  />
                </td>
                <td className="rating-col">{formatNumber(row.ratingPoints)}</td>
                <td className="rank-col">{row.rank ?? '-'}</td>
                <td className="fc-col">{row.fc ?? '-'}</td>
                <td className="sync-col">{row.sync ?? '-'}</td>
                <td className="dx-col">
                  {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
                </td>
                <td className="last-played-col">{row.latestPlayedAtLabel ?? toDateLabel(row.latestPlayedAtUnix) ?? '-'}</td>
                <td className="play-count-col">{formatNumber(row.playCount)}</td>
                <td className="version-col">{formatVersionLabel(row.version)}</td>
              </tr>
            ))}
          </tbody>
        </table>
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
  onOpenHistory,
}: RatingPageProps) {
  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">
        {sidebarTopContent}
        <section className="panel filter-panel">
          <div className="panel-heading compact">
            <div>
              <h2>RATING</h2>
              <p>NEW 상위 15곡과 OLD 상위 35곡의 레이팅 합계입니다. 보면상수가 알려지지 않은 곡의 경우 계산값이 잘못될 수 있습니다.</p>
            </div>
          </div>
          <div className="rating-stat-grid">
            <div className="rating-stat-card">
              <span>Current Rating</span>
              <strong>{formatNumber(ratingTotal)}</strong>
              <small className="rating-stat-sub">avg {formatRatingAvg(ratingTotal, newRows.length + oldRows.length)}</small>
            </div>
            <div className="rating-stat-card">
              <span>NEW TOP 15</span>
              <strong>{formatNumber(newRatingTotal)}</strong>
              <small className="rating-stat-sub">avg {formatRatingAvg(newRatingTotal, newRows.length)}, ~{formatRatingProjection(newRatingTotal, newRows.length)}</small>
            </div>
            <div className="rating-stat-card">
              <span>OLD TOP 35</span>
              <strong>{formatNumber(oldRatingTotal)}</strong>
              <small className="rating-stat-sub">avg {formatRatingAvg(oldRatingTotal, oldRows.length)}, ~{formatRatingProjection(oldRatingTotal, oldRows.length)}</small>
            </div>
          </div>
        </section>
      </aside>

      <div className="table-column rating-table-column">
        <RatingTable
          title="NEW"
          description="NEW 분류에서 레이팅이 높은 15곡"
          rows={newRows}
          songInfoUrl={songInfoUrl}
          onOpenSongDetail={onOpenSongDetail}
          onOpenHistory={onOpenHistory}
        />
        <RatingTable
          title="OLD"
          description="OLD 분류에서 레이팅이 높은 35곡"
          rows={oldRows}
          songInfoUrl={songInfoUrl}
          onOpenSongDetail={onOpenSongDetail}
          onOpenHistory={onOpenHistory}
        />
      </div>
    </div>
  );
}
