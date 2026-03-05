import type { ReactNode } from 'react';

import { toDateLabel } from '../derive';
import { formatNumber, formatPercent, formatVersionLabel } from '../app/utils';
import type { ScoreRow } from '../types';
import { ChartTypeLabel } from './ChartTypeLabel';
import { Jacket } from './Jacket';

interface RatingPageProps {
  sidebarTopContent?: ReactNode;
  songInfoUrl: string;
  ratingTotal: number;
  newRatingTotal: number;
  oldRatingTotal: number;
  newRows: ScoreRow[];
  oldRows: ScoreRow[];
}

function RatingTable({
  title,
  description,
  rows,
  songInfoUrl,
}: {
  title: string;
  description: string;
  rows: ScoreRow[];
  songInfoUrl: string;
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
                <td className="title-col">{row.title}</td>
                <td className="chart-col">
                  <ChartTypeLabel chartType={row.chartType} />
                </td>
                <td className="level-col">{row.internalLevel?.toFixed(1) ?? '-'}</td>
                <td className="achievement-col">
                  {row.achievementPercent === null ? '-' : formatPercent(row.achievementPercent)}
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
}: RatingPageProps) {
  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">
        {sidebarTopContent}
        <section className="panel filter-panel">
          <div className="panel-heading compact">
            <div>
              <h2>RATING</h2>
              <p>NEW 상위 15곡과 OLD 상위 35곡의 레이팅 합계입니다.</p>
            </div>
          </div>
          <div className="rating-stat-grid">
            <div className="rating-stat-card">
              <span>Current Rating</span>
              <strong>{formatNumber(ratingTotal)}</strong>
            </div>
            <div className="rating-stat-card">
              <span>NEW TOP 15</span>
              <strong>{formatNumber(newRatingTotal)}</strong>
            </div>
            <div className="rating-stat-card">
              <span>OLD TOP 35</span>
              <strong>{formatNumber(oldRatingTotal)}</strong>
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
        />
        <RatingTable
          title="OLD"
          description="OLD 분류에서 레이팅이 높은 35곡"
          rows={oldRows}
          songInfoUrl={songInfoUrl}
        />
      </div>
    </div>
  );
}
