import type { Dispatch, ReactNode, SetStateAction } from 'react';

import { toDateLabel } from '../derive';
import type {
  ChartType,
  DifficultyCategory,
  FcStatus,
  ScoreRow,
  SyncStatus,
} from '../types';
import type { ScoreSortKey } from '../app/constants';
import {
  formatVersionLabel,
  formatNumber,
  formatPercent,
  sortIndicator,
  toggleArrayValue,
} from '../app/utils';
import { ChartTypeLabel, getChartTypeToneClass } from './ChartTypeLabel';
import { DifficultyLabel, getDifficultyToneClass } from './DifficultyLabel';
import { Jacket } from './Jacket';
import { ToggleGroup } from './ToggleGroup';

interface ScoreExplorerSectionProps {
  sidebarTopContent?: ReactNode;
  scoreCountLabel: string;
  isLoading: boolean;
  showJackets: boolean;
  setShowJackets: Dispatch<SetStateAction<boolean>>;
  query: string;
  setQuery: Dispatch<SetStateAction<string>>;
  chartTypes: ChartType[];
  chartFilter: ChartType[];
  setChartFilter: Dispatch<SetStateAction<ChartType[]>>;
  difficulties: DifficultyCategory[];
  difficultyFilter: DifficultyCategory[];
  setDifficultyFilter: Dispatch<SetStateAction<DifficultyCategory[]>>;
  versionOptions: string[];
  versionSelection: string;
  setVersionSelection: Dispatch<SetStateAction<string>>;
  scoreRankOptions: string[];
  rankFilter: string[];
  onToggleRankFilter: (value: string) => void;
  fcOptions: FcStatus[];
  fcFilter: FcStatus[];
  setFcFilter: Dispatch<SetStateAction<FcStatus[]>>;
  syncOptions: SyncStatus[];
  syncFilter: SyncStatus[];
  setSyncFilter: Dispatch<SetStateAction<SyncStatus[]>>;
  achievementMin: number;
  setAchievementMin: Dispatch<SetStateAction<number>>;
  achievementMax: number;
  setAchievementMax: Dispatch<SetStateAction<number>>;
  internalMin: number;
  setInternalMin: Dispatch<SetStateAction<number>>;
  internalMax: number;
  setInternalMax: Dispatch<SetStateAction<number>>;
  daysMin: number;
  setDaysMin: Dispatch<SetStateAction<number>>;
  daysMax: number;
  setDaysMax: Dispatch<SetStateAction<number>>;
  filteredScoreRows: ScoreRow[];
  songInfoUrl: string;
  onOpenSongDetail: (row: ScoreRow) => void;
  onOpenHistory: (row: ScoreRow) => void;
  scoreSortKey: ScoreSortKey;
  scoreSortDesc: boolean;
  onSortBy: (key: ScoreSortKey) => void;
}

export function ScoreExplorerSection({
  sidebarTopContent,
  scoreCountLabel,
  isLoading,
  showJackets,
  setShowJackets,
  query,
  setQuery,
  chartTypes,
  chartFilter,
  setChartFilter,
  difficulties,
  difficultyFilter,
  setDifficultyFilter,
  versionOptions,
  versionSelection,
  setVersionSelection,
  scoreRankOptions,
  rankFilter,
  onToggleRankFilter,
  fcOptions,
  fcFilter,
  setFcFilter,
  syncOptions,
  syncFilter,
  setSyncFilter,
  achievementMin,
  setAchievementMin,
  achievementMax,
  setAchievementMax,
  internalMin,
  setInternalMin,
  internalMax,
  setInternalMax,
  daysMin,
  setDaysMin,
  daysMax,
  setDaysMax,
  filteredScoreRows,
  songInfoUrl,
  onOpenSongDetail,
  onOpenHistory,
  scoreSortKey,
  scoreSortDesc,
  onSortBy,
}: ScoreExplorerSectionProps) {
  const renderInternalLevel = (row: ScoreRow) => {
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

  const renderLevelCell = (row: ScoreRow) => {
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
              <span>검색 (곡명/버전/레벨)</span>
              <input
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="예: VERTeX, PRiSM, 14+"
              />
            </label>

            <ToggleGroup
              label="Chart Type"
              options={chartTypes}
              selected={chartFilter}
              onToggle={(value) => setChartFilter((prev) => toggleArrayValue(prev, value))}
              optionClassName={(value) => `chart-type-chip ${getChartTypeToneClass(value)}`}
            />

            <ToggleGroup
              label="Difficulty"
              options={difficulties}
              selected={difficultyFilter}
              onToggle={(value) => setDifficultyFilter((prev) => toggleArrayValue(prev, value))}
              renderLabel={(value) => <DifficultyLabel difficulty={value} short />}
              optionClassName={(value) => `difficulty-chip ${getDifficultyToneClass(value)}`}
            />
            <label>
              <span>Version</span>
              <select
                value={versionSelection}
                onChange={(event) => setVersionSelection(event.target.value)}
              >
                <option value="ALL">ALL</option>
                <option value="NEW">NEW</option>
                <option value="OLD">OLD</option>
                {versionOptions.map((version) => (
                  <option key={version} value={version}>
                    {formatVersionLabel(version)}
                  </option>
                ))}
              </select>
            </label>

            <ToggleGroup
              label="Rank"
              options={scoreRankOptions}
              selected={rankFilter}
              onToggle={onToggleRankFilter}
            />

            <ToggleGroup
              label="FC"
              options={fcOptions}
              selected={fcFilter}
              onToggle={(value) => setFcFilter((prev) => toggleArrayValue(prev, value))}
            />

            <ToggleGroup
              label="Sync"
              options={syncOptions}
              selected={syncFilter}
              onToggle={(value) => setSyncFilter((prev) => toggleArrayValue(prev, value))}
            />

            <div className="range-grid">
              <label>
                <span>달성률 최소</span>
                <input
                  type="number"
                  value={achievementMin}
                  min={0}
                  max={101}
                  step={0.0001}
                  onChange={(event) => setAchievementMin(Number(event.target.value))}
                />
              </label>
              <label>
                <span>달성률 최대</span>
                <input
                  type="number"
                  value={achievementMax}
                  min={0}
                  max={101}
                  step={0.0001}
                  onChange={(event) => setAchievementMax(Number(event.target.value))}
                />
              </label>
              <label>
                <span>내부레벨 최소</span>
                <input
                  type="number"
                  value={internalMin}
                  min={1}
                  max={15.5}
                  step={0.1}
                  onChange={(event) => setInternalMin(Number(event.target.value))}
                />
              </label>
              <label>
                <span>내부레벨 최대</span>
                <input
                  type="number"
                  value={internalMax}
                  min={1}
                  max={15.5}
                  step={0.1}
                  onChange={(event) => setInternalMax(Number(event.target.value))}
                />
              </label>
              <label>
                <span>경과일 최소</span>
                <input
                  type="number"
                  value={daysMin}
                  min={0}
                  max={5000}
                  step={1}
                  onChange={(event) => setDaysMin(Number(event.target.value))}
                />
              </label>
              <label>
                <span>경과일 최대</span>
                <input
                  type="number"
                  value={daysMax}
                  min={0}
                  max={5000}
                  step={1}
                  onChange={(event) => setDaysMax(Number(event.target.value))}
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
              <h2>Charts</h2>
              <p>점수 데이터와 차트 메타데이터를 함께 확인합니다. 회색 소수점은 추정 내부레벨입니다.</p>
            </div>
            <div className="panel-heading-actions">
              <div className="view-mode-switch" role="group" aria-label="Charts layout">
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
              <span className="panel-count">{scoreCountLabel}</span>
            </div>
          </div>
          <div className="table-wrap">
            {isLoading ? <div className="table-loading-state">Loading charts...</div> : null}
            <table className="score-table compact-table">
              <thead>
                <tr>
                  {showJackets ? <th className="jacket-col">Jacket</th> : null}
                  <th className="sortable title-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('title')}>
                      <span>Title</span>
                      <span className="sort-indicator">{sortIndicator(scoreSortKey === 'title', scoreSortDesc)}</span>
                    </button>
                  </th>
                  <th className="chart-col">Chart</th>
                  <th className="sortable level-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('internal')}>
                      <span>Lv</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'internal', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable achievement-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('achievement')}>
                      <span>Achv</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'achievement', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable rating-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('rating')}>
                      <span>Rating</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'rating', scoreSortDesc)}
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
                        {sortIndicator(scoreSortKey === 'dxRatio', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable last-played-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('lastPlayed')}>
                      <span>Last Played</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'lastPlayed', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable play-count-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playCount')}>
                      <span>Play count</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'playCount', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="version-col">Version</th>
                </tr>
              </thead>
              <tbody>
                {filteredScoreRows.map((row) => (
                  <tr key={row.key}>
                    {showJackets ? (
                      <td className="jacket-col">
                        <Jacket songInfoUrl={songInfoUrl} imageName={row.imageName} title={row.title} />
                      </td>
                    ) : null}
                    <td className="title-col">
                      <div className="title-cell">
                        <button
                          type="button"
                          className="link-button"
                          onClick={() => void onOpenSongDetail(row)}
                        >
                          {row.title}
                        </button>
                      </div>
                    </td>
                    <td className="chart-col">
                      <ChartTypeLabel chartType={row.chartType} />
                    </td>
                    <td className="level-col">{renderLevelCell(row)}</td>
                    <td className="achievement-col">
                      {row.achievementPercent === null ? (
                        '-'
                      ) : (
                        <button
                          type="button"
                          className="achievement-history-button"
                          onClick={() => onOpenHistory(row)}
                        >
                          {formatPercent(row.achievementPercent)}
                        </button>
                      )}
                    </td>
                    <td className="rating-col">{formatNumber(row.ratingPoints)}</td>
                    <td className="rank-col">{row.rank ?? '-'}</td>
                    <td className="fc-col">{row.fc ?? '-'}</td>
                    <td className="sync-col">{row.sync ?? '-'}</td>
                    <td className="dx-col">
                      {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
                    </td>
                    <td
                      className="last-played-col"
                      title={row.daysSinceLastPlayed === null ? undefined : `${row.daysSinceLastPlayed}일 전`}
                    >
                      {row.latestPlayedAtLabel ?? toDateLabel(row.latestPlayedAtUnix) ?? '-'}
                    </td>
                    <td className="play-count-col">{formatNumber(row.playCount)}</td>
                    <td className="version-col">{formatVersionLabel(row.version)}</td>
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
