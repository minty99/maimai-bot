import type { Dispatch, SetStateAction } from 'react';

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
  formatDifficultyShort,
  formatNumber,
  formatPercent,
  formatRatio,
  sortIndicator,
  toggleArrayValue,
} from '../app/utils';
import { Jacket } from './Jacket';
import { ToggleGroup } from './ToggleGroup';

interface ScoreExplorerSectionProps {
  scoreCountLabel: string;
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
  includeNoAchievement: boolean;
  setIncludeNoAchievement: Dispatch<SetStateAction<boolean>>;
  includeNoInternalLevel: boolean;
  setIncludeNoInternalLevel: Dispatch<SetStateAction<boolean>>;
  includeNeverPlayed: boolean;
  setIncludeNeverPlayed: Dispatch<SetStateAction<boolean>>;
  filteredScoreRows: ScoreRow[];
  songInfoUrl: string;
  onOpenSongDetail: (title: string) => Promise<void>;
  scoreSortKey: ScoreSortKey;
  scoreSortDesc: boolean;
  onSortBy: (key: ScoreSortKey) => void;
}

export function ScoreExplorerSection({
  scoreCountLabel,
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
  includeNoAchievement,
  setIncludeNoAchievement,
  includeNoInternalLevel,
  setIncludeNoInternalLevel,
  includeNeverPlayed,
  setIncludeNeverPlayed,
  filteredScoreRows,
  songInfoUrl,
  onOpenSongDetail,
  scoreSortKey,
  scoreSortDesc,
  onSortBy,
}: ScoreExplorerSectionProps) {
  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">
        <section className="panel filter-panel">
          <h2>Score Filters</h2>
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
            />

            <ToggleGroup
              label="Difficulty"
              options={difficulties}
              selected={difficultyFilter}
              onToggle={(value) => setDifficultyFilter((prev) => toggleArrayValue(prev, value))}
              formatLabel={formatDifficultyShort}
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
                    {version}
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

            <div className="toggle-grid">
              <label>
                <input
                  type="checkbox"
                  checked={includeNoAchievement}
                  onChange={(event) => setIncludeNoAchievement(event.target.checked)}
                />
                달성률 없는 차트 포함
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={includeNoInternalLevel}
                  onChange={(event) => setIncludeNoInternalLevel(event.target.checked)}
                />
                내부레벨 없는 차트 포함
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={includeNeverPlayed}
                  onChange={(event) => setIncludeNeverPlayed(event.target.checked)}
                />
                최근 로그에 없는 차트 포함
              </label>
            </div>
          </div>
        </section>
      </aside>

      <div className="table-column">
        <section className="panel">
          <h2>Score Table ({scoreCountLabel})</h2>
          <div className="table-wrap">
            <table className="score-table compact-table">
              <thead>
                <tr>
                  <th>Jacket</th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('title')}>
                      <span>Title</span>
                      <span className="sort-indicator">{sortIndicator(scoreSortKey === 'title', scoreSortDesc)}</span>
                    </button>
                  </th>
                  <th>Chart</th>
                  <th>Diff</th>
                  <th>Lv</th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('internal')}>
                      <span>IntLv</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'internal', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('achievement')}>
                      <span>Achv</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'achievement', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('rating')}>
                      <span>Rating</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'rating', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th>Rank</th>
                  <th>FC</th>
                  <th>Sync</th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('dxRatio')}>
                      <span>DX</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'dxRatio', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('lastPlayed')}>
                      <span>Last Played</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'lastPlayed', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playCount')}>
                      <span>Play count</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'playCount', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th>Version</th>
                </tr>
              </thead>
              <tbody>
                {filteredScoreRows.map((row) => (
                  <tr key={row.key}>
                    <td>
                      <Jacket songInfoUrl={songInfoUrl} imageName={row.imageName} title={row.title} />
                    </td>
                    <td>
                      <div className="title-cell">
                        <button
                          type="button"
                          className="link-button"
                          onClick={() => void onOpenSongDetail(row.title)}
                        >
                          {row.title}
                        </button>
                        {row.userLevel ? <span className="muted">유저레벨: {row.userLevel}</span> : null}
                      </div>
                    </td>
                    <td>{row.chartType}</td>
                    <td>{formatDifficultyShort(row.difficulty)}</td>
                    <td>{row.level ?? '-'}</td>
                    <td>{row.internalLevel?.toFixed(1) ?? '-'}</td>
                    <td>{formatPercent(row.achievementPercent)}</td>
                    <td>{formatNumber(row.ratingPoints)}</td>
                    <td>{row.rank ?? '-'}</td>
                    <td>{row.fc ?? '-'}</td>
                    <td>{row.sync ?? '-'}</td>
                    <td>
                      {formatNumber(row.dxScore)} / {formatNumber(row.dxScoreMax)}
                      <div className="muted">{formatRatio(row.dxRatio)}</div>
                    </td>
                    <td title={row.daysSinceLastPlayed === null ? undefined : `${row.daysSinceLastPlayed}일 전`}>
                      {row.latestPlayedAtLabel ?? toDateLabel(row.latestPlayedAtUnix) ?? '-'}
                    </td>
                    <td>{formatNumber(row.playCount)}</td>
                    <td>{row.version ?? '-'}</td>
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
