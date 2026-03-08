import type { Dispatch, ReactNode, SetStateAction } from 'react';

import type { PlaylogSortKey } from '../app/constants';
import {
  formatNumber,
  formatPercent,
  sortIndicator,
  toggleArrayValue,
} from '../app/utils';
import type { ChartType, DifficultyCategory, PlaylogRow } from '../types';
import { toDateLabel } from '../derive';
import { ChartTypeLabel, getChartTypeToneClass } from './ChartTypeLabel';
import { DifficultyLabel, getDifficultyToneClass } from './DifficultyLabel';
import { Jacket } from './Jacket';
import { ToggleGroup } from './ToggleGroup';

interface PlaylogExplorerSectionProps {
  sidebarTopContent?: ReactNode;
  playlogCountLabel: string;
  showJackets: boolean;
  setShowJackets: Dispatch<SetStateAction<boolean>>;
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
  isPlaylogDateFilterDisabled: boolean;
  setIsPlaylogDateFilterDisabled: Dispatch<SetStateAction<boolean>>;
  selectedPlaylogDayKey: string | null;
  setSelectedPlaylogDayKey: Dispatch<SetStateAction<string | null>>;
  playlogDayOptions: Array<{
    key: string;
    creditCount: number | null;
  }>;
  selectedPlaylogDayCreditCount: number | null;
  selectedPlaylogDaySongCount: number;
  filteredPlaylogRows: PlaylogRow[];
  songInfoUrl: string;
  playlogSortKey: PlaylogSortKey;
  playlogSortDesc: boolean;
  onSortBy: (key: PlaylogSortKey) => void;
}

export function PlaylogExplorerSection({
  sidebarTopContent,
  playlogCountLabel,
  showJackets,
  setShowJackets,
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
  isPlaylogDateFilterDisabled,
  setIsPlaylogDateFilterDisabled,
  selectedPlaylogDayKey,
  setSelectedPlaylogDayKey,
  playlogDayOptions,
  selectedPlaylogDayCreditCount,
  selectedPlaylogDaySongCount,
  filteredPlaylogRows,
  songInfoUrl,
  playlogSortKey,
  playlogSortDesc,
  onSortBy,
}: PlaylogExplorerSectionProps) {
  const renderInternalLevel = (row: PlaylogRow) => {
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

  const renderLevelCell = (row: PlaylogRow) => {
    if (row.internalLevel === null) {
      return '-';
    }

    return (
      <span className={`level-badge ${getDifficultyToneClass(row.difficulty)}`}>
        {renderInternalLevel(row)}
      </span>
    );
  };

  const renderAchievementCell = (row: PlaylogRow) => {
    return (
      <span className={`achievement-value ${row.isNewRecord ? 'achievement-value--new' : ''}`}>
        {formatPercent(row.achievementPercent)}
      </span>
    );
  };

  const handlePlaylogDayInputChange = (value: string) => {
    if (!value) {
      return;
    }
    if (!playlogDayOptions.some((option) => option.key === value)) {
      return;
    }
    setSelectedPlaylogDayKey(value);
  };

  const formatDayLabel = (dayKey: string) => dayKey.replace(/-/g, '/');

  const playlogDaySummary = isPlaylogDateFilterDisabled
    ? `전체: ${selectedPlaylogDaySongCount.toLocaleString()}곡`
    : `${selectedPlaylogDaySongCount.toLocaleString()}곡 · ${selectedPlaylogDayCreditCount === null ? '-' : selectedPlaylogDayCreditCount.toLocaleString()} 크레딧`;

  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">
        {sidebarTopContent}
        <section className="panel filter-panel">
          <div className="panel-heading compact">
            <div>
              <h2>Filters</h2>
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

            <div className="playlog-day-filter">
              <label className="playlog-day-toggle">
                <input
                  type="checkbox"
                  checked={isPlaylogDateFilterDisabled}
                  onChange={(event) => setIsPlaylogDateFilterDisabled(event.target.checked)}
                />
                <span>전체 플레이 기록 보기</span>
              </label>
              <label className="search-box">
                <span>플레이 날짜 (maimai day 04:00 기준)</span>
                <select
                  value={selectedPlaylogDayKey ?? ''}
                  onChange={(event) => handlePlaylogDayInputChange(event.target.value)}
                  disabled={isPlaylogDateFilterDisabled || playlogDayOptions.length === 0}
                >
                  {playlogDayOptions.map((option) => (
                    <option key={option.key} value={option.key}>
                      {formatDayLabel(option.key)} ({option.creditCount === null ? '-' : option.creditCount.toLocaleString()} credits)
                    </option>
                  ))}
                </select>
              </label>
              <p className="playlog-day-filter-summary">{playlogDaySummary}</p>
            </div>

            <ToggleGroup
              label="Chart Type"
              options={chartTypes}
              selected={playlogChartFilter}
              onToggle={(value) => setPlaylogChartFilter((prev) => toggleArrayValue(prev, value))}
              optionClassName={(value) => `chart-type-chip ${getChartTypeToneClass(value)}`}
            />

            <ToggleGroup
              label="Difficulty"
              options={difficulties}
              selected={playlogDifficultyFilter}
              onToggle={(value) => setPlaylogDifficultyFilter((prev) => toggleArrayValue(prev, value))}
              renderLabel={(value) => <DifficultyLabel difficulty={value} short />}
              optionClassName={(value) => `difficulty-chip ${getDifficultyToneClass(value)}`}
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

          </div>
        </section>
      </aside>

      <div className="table-column">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>Playlogs</h2>
            </div>
            <div className="panel-heading-actions">
              <div className="view-mode-switch" role="group" aria-label="Playlogs layout">
                <button
                  type="button"
                  className={showJackets ? 'active' : ''}
                  onClick={() => setShowJackets(true)}
                >
                  Jacket
                </button>
                <button
                  type="button"
                  className={!showJackets ? 'active' : ''}
                  onClick={() => setShowJackets(false)}
                >
                  Compact
                </button>
              </div>
              <span className="panel-count">{playlogCountLabel}</span>
            </div>
          </div>
          <div className="table-wrap">
            <table className="playlog-table compact-table">
              <thead>
                <tr>
                  <th className="sortable credit-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playCount')}>
                      <span>Credit #</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'playCount', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
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
                  <th className="level-col">Lv</th>
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
                </tr>
              </thead>
              <tbody>
                {filteredPlaylogRows.map((row) => (
                  <tr key={row.key}>
                    <td className="credit-col">{row.creditId ?? '-'}</td>
                    {showJackets ? (
                      <td className="jacket-col">
                        <Jacket songInfoUrl={songInfoUrl} imageName={row.imageName} title={row.title} />
                      </td>
                    ) : null}
                    <td className="played-at-col">{row.playedAtLabel ?? toDateLabel(row.playedAtUnix) ?? '-'}</td>
                    <td className="track-col">{row.track ?? '-'}</td>
                    <td className="title-col">{row.title}</td>
                    <td className="chart-col">
                      <ChartTypeLabel chartType={row.chartType} />
                    </td>
                    <td className="level-col">{renderLevelCell(row)}</td>
                    <td className="achievement-col">{renderAchievementCell(row)}</td>
                    <td className="rating-col">{formatNumber(row.ratingPoints)}</td>
                    <td className="rank-col">{row.rank ?? '-'}</td>
                    <td className="fc-col">{row.fc ?? '-'}</td>
                    <td className="sync-col">{row.sync ?? '-'}</td>
                    <td className="dx-col">
                      {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
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
