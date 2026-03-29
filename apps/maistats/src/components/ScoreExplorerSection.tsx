import {
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { SearchInput } from './SearchInput';
import { useVirtualizer } from '@tanstack/react-virtual';

import { useI18n } from '../app/i18n';
import { toDateLabel, toIntegerRating } from '../derive';
import type {
  ChartType,
  DifficultyCategory,
  ScoreRow,
} from '../types';
import type { ScoreSortKey } from '../app/constants';
import {
  formatAliasSummary,
  formatVersionLabel,
  formatNumber,
  sortIndicator,
  toggleArrayValue,
} from '../app/utils';
import { ChartTypeLabel, getChartTypeToneClass } from './ChartTypeLabel';
import { DifficultyLabel, getDifficultyToneClass } from './DifficultyLabel';
import { Jacket } from './Jacket';
import { LevelCell } from './LevelCell';
import type { SongDetailTarget } from './TableActionCells';
import { AchievementHistoryButton, SongTitleButton } from './TableActionCells';
import { ToggleGroup } from './ToggleGroup';

interface ScoreExplorerSectionProps {
  sidebarTopContent?: ReactNode;
  scoreCountLabel: string;
  isLoading: boolean;
  showJackets: boolean;
  setShowJackets: Dispatch<SetStateAction<boolean>>;
  appliedQuery: string;
  onApplyQuery: (query: string) => void;
  chartTypes: ChartType[];
  chartFilter: ChartType[];
  setChartFilter: Dispatch<SetStateAction<ChartType[]>>;
  difficulties: DifficultyCategory[];
  difficultyFilter: DifficultyCategory[];
  setDifficultyFilter: Dispatch<SetStateAction<DifficultyCategory[]>>;
  versionOptions: string[];
  versionSelection: string;
  setVersionSelection: Dispatch<SetStateAction<string>>;
  playedOnly: boolean;
  setPlayedOnly: Dispatch<SetStateAction<boolean>>;
  internalLevelPresetOptions: string[];
  selectedInternalLevelPresets: string[];
  onToggleInternalLevelPreset: (value: string) => void;
  scoreRankOptions: string[];
  selectedScoreRankPresets: string[];
  onToggleScoreRankPreset: (value: string) => void;
  fcOptions: string[];
  fcFilter: string[];
  onToggleFcFilter: (value: string) => void;
  syncOptions: string[];
  syncFilter: string[];
  onToggleSyncFilter: (value: string) => void;
  achievementMin: number;
  onChangeAchievementMin: (value: number) => void;
  achievementMax: number;
  onChangeAchievementMax: (value: number) => void;
  internalMin: number;
  onChangeInternalMin: (value: number) => void;
  internalMax: number;
  onChangeInternalMax: (value: number) => void;
  daysMin: number;
  setDaysMin: Dispatch<SetStateAction<number>>;
  daysMax: number;
  setDaysMax: Dispatch<SetStateAction<number>>;
  filteredScoreRows: ScoreRow[];
  songInfoUrl: string;
  onOpenSongDetail: (target: SongDetailTarget) => void;
  onOpenHistory: (row: ScoreRow) => void;
  scoreSortKey: ScoreSortKey;
  scoreSortDesc: boolean;
  onSortBy: (key: ScoreSortKey) => void;
  onResetFilters: () => void;
}

export function ScoreExplorerSection({
  sidebarTopContent,
  scoreCountLabel,
  isLoading,
  showJackets,
  setShowJackets,
  appliedQuery,
  onApplyQuery,
  chartTypes,
  chartFilter,
  setChartFilter,
  difficulties,
  difficultyFilter,
  setDifficultyFilter,
  versionOptions,
  versionSelection,
  setVersionSelection,
  playedOnly,
  setPlayedOnly,
  internalLevelPresetOptions,
  selectedInternalLevelPresets,
  onToggleInternalLevelPreset,
  scoreRankOptions,
  selectedScoreRankPresets,
  onToggleScoreRankPreset,
  fcOptions,
  fcFilter,
  onToggleFcFilter,
  syncOptions,
  syncFilter,
  onToggleSyncFilter,
  achievementMin,
  onChangeAchievementMin,
  achievementMax,
  onChangeAchievementMax,
  internalMin,
  onChangeInternalMin,
  internalMax,
  onChangeInternalMax,
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
  onResetFilters,
}: ScoreExplorerSectionProps) {
  const { locale, t } = useI18n();
  const tableWrapRef = useRef<HTMLDivElement | null>(null);
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false);

  const virtualizer = useVirtualizer({
    count: filteredScoreRows.length,
    getScrollElement: () => tableWrapRef.current,
    estimateSize: () => (showJackets ? 80 : 36),
    overscan: 10,
  });

  useEffect(() => {
    if (tableWrapRef.current) tableWrapRef.current.scrollTop = 0;
  }, [filteredScoreRows, showJackets]);

  const virtualItems = virtualizer.getVirtualItems();
  const colCount = showJackets ? 13 : 12;
  const paddingTop = virtualItems[0]?.start ?? 0;
  const paddingBottom =
    virtualItems.length > 0
      ? virtualizer.getTotalSize() - virtualItems[virtualItems.length - 1].end
      : 0;

  const filterPanel = (
    <section className="panel filter-panel">
      <div className="panel-heading compact">
        <div>
          <h2>{t('common.filters')}</h2>
        </div>
        <button type="button" className="filter-reset-button" onClick={onResetFilters}>
          {t('scores.resetAll')}
        </button>
      </div>
      <div className="filter-grid">
        <SearchInput
          label={t('scores.searchLabel')}
          placeholder={t('scores.searchPlaceholder')}
          appliedQuery={appliedQuery}
          onApplyQuery={onApplyQuery}
        />

        <ToggleGroup
          label={t('scores.chartType')}
          options={chartTypes}
          selected={chartFilter}
          onToggle={(value) => setChartFilter((prev) => toggleArrayValue(prev, value))}
          optionClassName={(value) => `chart-type-chip ${getChartTypeToneClass(value)}`}
        />

        <ToggleGroup
          label={t('scores.difficulty')}
          options={difficulties}
          selected={difficultyFilter}
          onToggle={(value) => setDifficultyFilter((prev) => toggleArrayValue(prev, value))}
          renderLabel={(value) => <DifficultyLabel difficulty={value} short />}
          optionClassName={(value) => `difficulty-chip ${getDifficultyToneClass(value)}`}
        />

        <div className="filter-block filter-block-select">
          <div className="filter-label">{t('scores.version')}</div>
          <label>
            <select
              value={versionSelection}
              onChange={(event) => setVersionSelection(event.target.value)}
            >
              <option value="ALL">{t('scores.versionAll')}</option>
              <option value="NEW">{t('scores.versionNew')}</option>
              <option value="OLD">{t('scores.versionOld')}</option>
              {versionOptions.map((version) => (
                <option key={version} value={version}>
                  {formatVersionLabel(version)}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="filter-block score-special-filters">
          <label className="score-special-toggle">
            <input
              type="checkbox"
              checked={playedOnly}
              onChange={(event) => setPlayedOnly(event.target.checked)}
            />
            <span>{t('scores.playedOnly')}</span>
          </label>
        </div>

        <div className="filter-block">
          <div className="filter-label">{t('scores.level')}</div>
          <div className="range-pair">
            <label>
              <input
                type="number"
                value={internalMin}
                min={1}
                max={15.5}
                step={0.1}
                aria-label={t('scores.levelMin')}
                onChange={(event) => onChangeInternalMin(Number(event.target.value))}
              />
            </label>
            <span className="range-separator">~</span>
            <label>
              <input
                type="number"
                value={internalMax}
                min={1}
                max={15.5}
                step={0.1}
                aria-label={t('scores.levelMax')}
                onChange={(event) => onChangeInternalMax(Number(event.target.value))}
              />
            </label>
          </div>
          <ToggleGroup
            label=""
            options={internalLevelPresetOptions}
            selected={selectedInternalLevelPresets}
            onToggle={onToggleInternalLevelPreset}
            hideLabel
          />
        </div>

        <div className="filter-block">
          <div className="filter-label">{t('scores.score')}</div>
          <div className="range-pair">
            <label>
              <input
                type="number"
                value={achievementMin}
                min={0}
                max={101}
                step={0.0001}
                aria-label={t('scores.achievementMin')}
                onChange={(event) => onChangeAchievementMin(Number(event.target.value))}
              />
            </label>
            <span className="range-separator">~</span>
            <label>
              <input
                type="number"
                value={achievementMax}
                min={0}
                max={101}
                step={0.0001}
                aria-label={t('scores.achievementMax')}
                onChange={(event) => onChangeAchievementMax(Number(event.target.value))}
              />
            </label>
          </div>
          <ToggleGroup
            label=""
            options={scoreRankOptions}
            selected={selectedScoreRankPresets}
            onToggle={onToggleScoreRankPreset}
            hideLabel
          />
        </div>

        <ToggleGroup
          label="FC"
          options={fcOptions}
          selected={fcFilter}
          onToggle={onToggleFcFilter}
        />

        <ToggleGroup
          label="Sync"
          options={syncOptions}
          selected={syncFilter}
          onToggle={onToggleSyncFilter}
        />

        <div className="filter-block">
          <div className="filter-label">{t('scores.daysSince')}</div>
          <div className="range-pair">
            <label>
              <input
                type="number"
                value={daysMin}
                min={0}
                max={5000}
                step={1}
                aria-label={t('scores.daysMin')}
                onChange={(event) => setDaysMin(Number(event.target.value))}
              />
            </label>
            <span className="range-separator">~</span>
            <label>
              <input
                type="number"
                value={daysMax}
                min={0}
                max={5000}
                step={1}
                aria-label={t('scores.daysMax')}
                onChange={(event) => setDaysMax(Number(event.target.value))}
              />
            </label>
          </div>
        </div>
      </div>
    </section>
  );

  return (
    <>
      <div className="explorer-layout">
      <aside className="sidebar-column">
        {sidebarTopContent}
        {filterPanel}
      </aside>

      <div className="table-column">
        <section className="panel explorer-table-panel">
          <div className="panel-heading">
            <div>
              <h2>{t('scores.chartsTitle')}</h2>
              <p>{t('scores.chartsDescription')}</p>
            </div>
            <div className="panel-heading-actions">
              <div className="view-mode-switch" role="group" aria-label={t('scores.layout')}>
                <button
                  type="button"
                  className={showJackets ? 'active' : ''}
                  onClick={() => setShowJackets(true)}
                >
                  {t('common.jacket')}
                </button>
                <button
                  type="button"
                  className={!showJackets ? 'active' : ''}
                  onClick={() => setShowJackets(false)}
                >
                  {t('common.compact')}
                </button>
              </div>
              <span className="panel-count">{scoreCountLabel}</span>
            </div>
          </div>
          <div className="table-wrap" ref={tableWrapRef}>
            {isLoading ? <div className="table-loading-state">{t('common.loadingCharts')}</div> : null}
            <table className="score-table compact-table">
              <thead>
                <tr>
                  {showJackets ? <th className="jacket-col">{t('common.jacket')}</th> : null}
                  <th className="sortable title-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('title')}>
                      <span>{t('common.title')}</span>
                      <span className="sort-indicator">{sortIndicator(scoreSortKey === 'title', scoreSortDesc)}</span>
                    </button>
                  </th>
                  <th className="chart-col">{t('common.chart')}</th>
                  <th className="sortable level-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('internal')}>
                      <span>{t('common.levelShort')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'internal', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable achievement-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('achievement')}>
                      <span>{t('common.achievementShort')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'achievement', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable rating-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('rating')}>
                      <span>{t('common.rating')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'rating', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="rank-col">{t('common.rank')}</th>
                  <th className="sortable fc-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('fc')}>
                      <span>FC</span>
                      <span className="sort-indicator">{sortIndicator(scoreSortKey === 'fc', scoreSortDesc)}</span>
                    </button>
                  </th>
                  <th className="sortable sync-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('sync')}>
                      <span>{t('common.sync')}</span>
                      <span className="sort-indicator">{sortIndicator(scoreSortKey === 'sync', scoreSortDesc)}</span>
                    </button>
                  </th>
                  <th className="sortable dx-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('dxRatio')}>
                      <span>{t('common.dx')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'dxRatio', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable last-played-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('lastPlayed')}>
                      <span>{t('common.lastPlayed')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'lastPlayed', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable play-count-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playCount')}>
                      <span>{t('common.playCount')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(scoreSortKey === 'playCount', scoreSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable version-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('version')}>
                      <span>{t('common.version')}</span>
                      <span className="sort-indicator">{sortIndicator(scoreSortKey === 'version', scoreSortDesc)}</span>
                    </button>
                  </th>
                </tr>
              </thead>
              <tbody>
                {paddingTop > 0 && (
                  <tr style={{ height: paddingTop }}>
                    <td colSpan={colCount} />
                  </tr>
                )}
                {virtualItems.map((virtualRow) => {
                  const row = filteredScoreRows[virtualRow.index];
                  return (
                    <tr key={row.key} data-index={virtualRow.index} ref={virtualizer.measureElement}>
                      {showJackets ? (
                        <td className="jacket-col">
                          <Jacket songInfoUrl={songInfoUrl} imageName={row.imageName} title={row.title} />
                        </td>
                      ) : null}
                      <td className="title-col">
                        <div className="title-cell">
                          <SongTitleButton
                            target={row}
                            title={row.title}
                            subtitle={showJackets ? formatAliasSummary(row.aliases) : null}
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
                      <td className="rating-col">{formatNumber(toIntegerRating(row.rating), locale)}</td>
                      <td className="rank-col">{row.rank ?? '-'}</td>
                      <td className="fc-col">{row.fc ?? '-'}</td>
                      <td className="sync-col">{row.sync ?? '-'}</td>
                      <td className="dx-col">
                        {formatNumber(row.dxScore, locale)} / {formatNumber(row.dxScoreMax, locale)}
                      </td>
                      <td
                        className="last-played-col"
                        title={row.daysSinceLastPlayed === null ? undefined : t('units.daysAgo', { count: row.daysSinceLastPlayed })}
                      >
                        {row.latestPlayedAtLabel ?? toDateLabel(row.latestPlayedAtUnix, locale) ?? '-'}
                      </td>
                      <td className="play-count-col">{formatNumber(row.playCount, locale)}</td>
                      <td className="version-col">{formatVersionLabel(row.version)}</td>
                    </tr>
                  );
                })}
                {paddingBottom > 0 && (
                  <tr style={{ height: paddingBottom }}>
                    <td colSpan={colCount} />
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </section>
      </div>
      </div>

      <button
        type="button"
        className="mobile-filter-fab"
        onClick={() => setIsFilterModalOpen(true)}
      >
        {t('common.filters')}
      </button>

      {isFilterModalOpen ? (
        <div className="modal-backdrop mobile-filter-backdrop" onClick={() => setIsFilterModalOpen(false)}>
          <section
            className="modal-card panel mobile-filter-modal"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="detail-header">
              <h2>{t('common.filters')}</h2>
              <button
                type="button"
                className="modal-close-button"
                onClick={() => setIsFilterModalOpen(false)}
              >
                {t('common.close')}
              </button>
            </div>
            {sidebarTopContent}
            {filterPanel}
          </section>
        </div>
      ) : null}
    </>
  );
}
