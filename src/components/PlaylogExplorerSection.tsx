import type { Dispatch, SetStateAction } from 'react';

import type { PlaylogSortKey } from '../app/constants';
import {
  formatDifficultyShort,
  formatNumber,
  formatPercent,
  sortIndicator,
  toggleArrayValue,
} from '../app/utils';
import type { ChartType, DifficultyCategory, PlaylogRow } from '../types';
import { toDateLabel } from '../derive';
import { Jacket } from './Jacket';
import { ToggleGroup } from './ToggleGroup';

interface PlaylogExplorerSectionProps {
  playlogCountLabel: string;
  showJackets: boolean;
  playlogQuery: string;
  setPlaylogQuery: Dispatch<SetStateAction<string>>;
  chartTypes: ChartType[];
  playlogChartFilter: ChartType[];
  setPlaylogChartFilter: Dispatch<SetStateAction<ChartType[]>>;
  difficulties: DifficultyCategory[];
  playlogDifficultyFilter: DifficultyCategory[];
  setPlaylogDifficultyFilter: Dispatch<SetStateAction<DifficultyCategory[]>>;
  playlogAchievementMin: number;
  setPlaylogAchievementMin: Dispatch<SetStateAction<number>>;
  playlogAchievementMax: number;
  setPlaylogAchievementMax: Dispatch<SetStateAction<number>>;
  playlogIncludeUnknownDiff: boolean;
  setPlaylogIncludeUnknownDiff: Dispatch<SetStateAction<boolean>>;
  playlogNewRecordOnly: boolean;
  setPlaylogNewRecordOnly: Dispatch<SetStateAction<boolean>>;
  playlogFirstPlayOnly: boolean;
  setPlaylogFirstPlayOnly: Dispatch<SetStateAction<boolean>>;
  filteredPlaylogRows: PlaylogRow[];
  songInfoUrl: string;
  playlogSortKey: PlaylogSortKey;
  playlogSortDesc: boolean;
  onSortBy: (key: PlaylogSortKey) => void;
}

export function PlaylogExplorerSection({
  playlogCountLabel,
  showJackets,
  playlogQuery,
  setPlaylogQuery,
  chartTypes,
  playlogChartFilter,
  setPlaylogChartFilter,
  difficulties,
  playlogDifficultyFilter,
  setPlaylogDifficultyFilter,
  playlogAchievementMin,
  setPlaylogAchievementMin,
  playlogAchievementMax,
  setPlaylogAchievementMax,
  playlogIncludeUnknownDiff,
  setPlaylogIncludeUnknownDiff,
  playlogNewRecordOnly,
  setPlaylogNewRecordOnly,
  playlogFirstPlayOnly,
  setPlaylogFirstPlayOnly,
  filteredPlaylogRows,
  songInfoUrl,
  playlogSortKey,
  playlogSortDesc,
  onSortBy,
}: PlaylogExplorerSectionProps) {
  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">
        <section className="panel filter-panel">
          <div className="panel-heading compact">
            <div>
              <h2>Filters</h2>
              <p>플레이로그, 기록 변화, 달성률 조건으로 이력을 좁힙니다.</p>
            </div>
          </div>
          <div className="filter-grid">
            <label className="search-box">
              <span>검색 (곡명/시각)</span>
              <input
                type="search"
                value={playlogQuery}
                onChange={(event) => setPlaylogQuery(event.target.value)}
                placeholder="예: 2026/02/25, BUDDiES"
              />
            </label>

            <ToggleGroup
              label="Chart Type"
              options={chartTypes}
              selected={playlogChartFilter}
              onToggle={(value) => setPlaylogChartFilter((prev) => toggleArrayValue(prev, value))}
            />

            <ToggleGroup
              label="Difficulty"
              options={difficulties}
              selected={playlogDifficultyFilter}
              onToggle={(value) => setPlaylogDifficultyFilter((prev) => toggleArrayValue(prev, value))}
              formatLabel={formatDifficultyShort}
            />

            <div className="range-grid compact">
              <label>
                <span>달성률 최소</span>
                <input
                  type="number"
                  value={playlogAchievementMin}
                  min={0}
                  max={101}
                  step={0.0001}
                  onChange={(event) => setPlaylogAchievementMin(Number(event.target.value))}
                />
              </label>
              <label>
                <span>달성률 최대</span>
                <input
                  type="number"
                  value={playlogAchievementMax}
                  min={0}
                  max={101}
                  step={0.0001}
                  onChange={(event) => setPlaylogAchievementMax(Number(event.target.value))}
                />
              </label>
            </div>

            <div className="toggle-grid">
              <label>
                <input
                  type="checkbox"
                  checked={playlogIncludeUnknownDiff}
                  onChange={(event) => setPlaylogIncludeUnknownDiff(event.target.checked)}
                />
                난이도 정보 없는 로그 포함
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={playlogNewRecordOnly}
                  onChange={(event) => setPlaylogNewRecordOnly(event.target.checked)}
                />
                New Record만 보기
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={playlogFirstPlayOnly}
                  onChange={(event) => setPlaylogFirstPlayOnly(event.target.checked)}
                />
                First Play만 보기
              </label>
            </div>
          </div>
        </section>
      </aside>

      <div className="table-column">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>Playlogs</h2>
              <p>최근 플레이 흐름과 기록 상승 여부를 한 번에 확인합니다.</p>
            </div>
            <span className="panel-count">{playlogCountLabel}</span>
          </div>
          <div className="table-wrap">
            <table className="playlog-table compact-table">
              <thead>
                <tr>
                  {showJackets ? <th className="jacket-col">Jacket</th> : null}
                  <th className="sortable played-at-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playedAt')}>
                      <span>Played At</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'playedAt', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="track-col">Track</th>
                  <th className="sortable title-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('title')}>
                      <span>Title</span>
                      <span className="sort-indicator">{sortIndicator(playlogSortKey === 'title', playlogSortDesc)}</span>
                    </button>
                  </th>
                  <th className="chart-col">Chart</th>
                  <th className="diff-col">Diff</th>
                  <th className="sortable achievement-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('achievement')}>
                      <span>Achv</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'achievement', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable rating-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('rating')}>
                      <span>Rating</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'rating', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="rank-col">Rank</th>
                  <th className="fc-col">FC</th>
                  <th className="sync-col">Sync</th>
                  <th className="sortable dx-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('dxRatio')}>
                      <span>DX</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'dxRatio', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable play-count-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playCount')}>
                      <span>Credit Count</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'playCount', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="flags-col">Flags</th>
                </tr>
              </thead>
              <tbody>
                {filteredPlaylogRows.map((row) => (
                  <tr key={row.key}>
                    {showJackets ? (
                      <td className="jacket-col">
                        <Jacket songInfoUrl={songInfoUrl} imageName={row.imageName} title={row.title} />
                      </td>
                    ) : null}
                    <td className="played-at-col">{row.playedAtLabel ?? toDateLabel(row.playedAtUnix) ?? '-'}</td>
                    <td className="track-col">{row.track ?? '-'}</td>
                    <td className="title-col">{row.title}</td>
                    <td className="chart-col">{row.chartType}</td>
                    <td className="diff-col">{formatDifficultyShort(row.difficulty)}</td>
                    <td className="achievement-col">{formatPercent(row.achievementPercent)}</td>
                    <td className="rating-col">{formatNumber(row.ratingPoints)}</td>
                    <td className="rank-col">{row.rank ?? '-'}</td>
                    <td className="fc-col">{row.fc ?? '-'}</td>
                    <td className="sync-col">{row.sync ?? '-'}</td>
                    <td className="dx-col">
                      {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
                    </td>
                    <td className="play-count-col">{row.creditPlayCount ?? '-'}</td>
                    <td className="flags-col">
                      {row.isNewRecord ? 'NEW ' : ''}
                      {row.isFirstPlay ? 'FIRST' : ''}
                      {!row.isNewRecord && !row.isFirstPlay ? '-' : ''}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </div>
  );
}
