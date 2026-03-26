import { useState, type Dispatch, type ReactNode, type SetStateAction } from 'react';
import { SearchInput } from './SearchInput';

import type { PlaylogSortKey } from '../app/constants';
import { useI18n } from '../app/i18n';
import {
  formatAliasSummary,
  formatNumber,
  sortIndicator,
  toggleArrayValue,
} from '../app/utils';
import type { ChartType, DifficultyCategory, PlaylogRow } from '../types';
import { toDateLabel, toIntegerRating } from '../derive';
import { ChartTypeLabel, getChartTypeToneClass } from './ChartTypeLabel';
import { DifficultyLabel, getDifficultyToneClass } from './DifficultyLabel';
import { Jacket } from './Jacket';
import { LevelCell } from './LevelCell';
import type { SongDetailTarget } from './TableActionCells';
import { AchievementHistoryButton, SongTitleButton } from './TableActionCells';
import { ToggleGroup } from './ToggleGroup';

interface PlaylogExplorerSectionProps {
  sidebarTopContent?: ReactNode;
  playlogCountLabel: string;
  isLoading: boolean;
  showJackets: boolean;
  setShowJackets: Dispatch<SetStateAction<boolean>>;
  appliedPlaylogQuery: string;
  onApplyPlaylogQuery: (query: string) => void;
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
  playlogBestOnly: boolean;
  setPlaylogBestOnly: Dispatch<SetStateAction<boolean>>;
  playlogNewRecordOnly: boolean;
  setPlaylogNewRecordOnly: Dispatch<SetStateAction<boolean>>;
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
  getSongDetailTarget: (row: PlaylogRow) => SongDetailTarget | null;
  canOpenHistory: (row: PlaylogRow) => boolean;
  onOpenSongDetail: (target: SongDetailTarget) => void;
  onOpenHistory: (row: PlaylogRow) => void;
  playlogSortKey: PlaylogSortKey;
  playlogSortDesc: boolean;
  onSortBy: (key: PlaylogSortKey) => void;
}

export function PlaylogExplorerSection({
  sidebarTopContent,
  playlogCountLabel,
  isLoading,
  showJackets,
  setShowJackets,
  appliedPlaylogQuery,
  onApplyPlaylogQuery,
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
  playlogBestOnly,
  setPlaylogBestOnly,
  playlogNewRecordOnly,
  setPlaylogNewRecordOnly,
  isPlaylogDateFilterDisabled,
  setIsPlaylogDateFilterDisabled,
  selectedPlaylogDayKey,
  setSelectedPlaylogDayKey,
  playlogDayOptions,
  selectedPlaylogDayCreditCount,
  selectedPlaylogDaySongCount,
  filteredPlaylogRows,
  songInfoUrl,
  getSongDetailTarget,
  canOpenHistory,
  onOpenSongDetail,
  onOpenHistory,
  playlogSortKey,
  playlogSortDesc,
  onSortBy,
}: PlaylogExplorerSectionProps) {
  const { formatNumber: formatLocalizedNumber, locale, t } = useI18n();
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false);

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
    ? t('playlogs.summaryAll', {
      songCount: formatLocalizedNumber(selectedPlaylogDaySongCount),
    })
    : t('playlogs.summaryDay', {
      songCount: formatLocalizedNumber(selectedPlaylogDaySongCount),
      creditCount: selectedPlaylogDayCreditCount === null
        ? '-'
        : formatLocalizedNumber(selectedPlaylogDayCreditCount),
    });

  const filterPanel = (
    <section className="panel filter-panel">
      <div className="panel-heading compact">
        <div>
          <h2>{t('common.filters')}</h2>
        </div>
      </div>
      <div className="filter-grid">
        <SearchInput
          label={t('playlogs.searchLabel')}
          placeholder={t('playlogs.searchPlaceholder')}
          appliedQuery={appliedPlaylogQuery}
          onApplyQuery={onApplyPlaylogQuery}
        />

        <div className="filter-block playlog-day-filter">
          <label className="playlog-day-toggle">
            <input
              type="checkbox"
              checked={isPlaylogDateFilterDisabled}
              onChange={(event) => setIsPlaylogDateFilterDisabled(event.target.checked)}
            />
            <span>{t('playlogs.showAll')}</span>
          </label>
          <label className="search-box">
            <span>{t('playlogs.dayLabel')}</span>
            <select
              value={selectedPlaylogDayKey ?? ''}
              onChange={(event) => handlePlaylogDayInputChange(event.target.value)}
              disabled={isPlaylogDateFilterDisabled || playlogDayOptions.length === 0}
            >
              {playlogDayOptions.map((option) => (
                <option key={option.key} value={option.key}>
                  {t('playlogs.dayOption', {
                    date: formatDayLabel(option.key),
                    credits: option.creditCount === null ? '-' : formatLocalizedNumber(option.creditCount),
                  })}
                </option>
              ))}
            </select>
          </label>
          <p className="playlog-day-filter-summary">{playlogDaySummary}</p>
        </div>

        <ToggleGroup
          label={t('scores.chartType')}
          options={chartTypes}
          selected={playlogChartFilter}
          onToggle={(value) => setPlaylogChartFilter((prev) => toggleArrayValue(prev, value))}
          optionClassName={(value) => `chart-type-chip ${getChartTypeToneClass(value)}`}
        />

        <ToggleGroup
          label={t('scores.difficulty')}
          options={difficulties}
          selected={playlogDifficultyFilter}
          onToggle={(value) => setPlaylogDifficultyFilter((prev) => toggleArrayValue(prev, value))}
          renderLabel={(value) => <DifficultyLabel difficulty={value} short />}
          optionClassName={(value) => `difficulty-chip ${getDifficultyToneClass(value)}`}
        />

        <div className="filter-block">
          <div className="range-grid compact">
            <label>
              <span>{t('scores.achievementMin')}</span>
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
              <span>{t('scores.achievementMax')}</span>
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

        <div className="playlog-special-filters filter-block">
          <label className="playlog-special-toggle">
            <input
              type="checkbox"
              checked={playlogBestOnly}
              onChange={(event) => setPlaylogBestOnly(event.target.checked)}
            />
            <span>{t('playlogs.bestOnly')}</span>
          </label>
          <label className="playlog-special-toggle">
            <input
              type="checkbox"
              checked={playlogNewRecordOnly}
              onChange={(event) => setPlaylogNewRecordOnly(event.target.checked)}
            />
            <span>{t('playlogs.newRecordOnly')}</span>
          </label>
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
              <h2>Playlogs</h2>
            </div>
            <div className="panel-heading-actions">
              <div className="view-mode-switch" role="group" aria-label={t('playlogs.layout')}>
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
              <span className="panel-count">{playlogCountLabel}</span>
            </div>
          </div>
          <div className="table-wrap">
            {isLoading ? <div className="table-loading-state">{t('common.loadingPlaylogs')}</div> : null}
            <table className="playlog-table compact-table">
              <thead>
                <tr>
                  <th className="sortable credit-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playCount')}>
                      <span>{t('playlogs.creditNumber')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'playCount', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  {showJackets ? <th className="jacket-col">{t('common.jacket')}</th> : null}
                  <th className="sortable played-at-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('playedAt')}>
                      <span>{t('playlogs.playedAt')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'playedAt', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="track-col">{t('common.track')}</th>
                  <th className="sortable title-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('title')}>
                      <span>{t('common.title')}</span>
                      <span className="sort-indicator">{sortIndicator(playlogSortKey === 'title', playlogSortDesc)}</span>
                    </button>
                  </th>
                  <th className="chart-col">{t('common.chart')}</th>
                  <th className="level-col">{t('common.levelShort')}</th>
                  <th className="sortable achievement-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('achievement')}>
                      <span>{t('common.achievementShort')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'achievement', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="sortable rating-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('rating')}>
                      <span>{t('common.rating')}</span>
                      <span className="sort-indicator">
                        {sortIndicator(playlogSortKey === 'rating', playlogSortDesc)}
                      </span>
                    </button>
                  </th>
                  <th className="rank-col">{t('common.rank')}</th>
                  <th className="fc-col">FC</th>
                  <th className="sync-col">{t('common.sync')}</th>
                  <th className="sortable dx-col">
                    <button type="button" className="th-sort-button" onClick={() => onSortBy('dxRatio')}>
                      <span>{t('common.dx')}</span>
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
                    <td className="played-at-col">{row.playedAtLabel ?? toDateLabel(row.playedAtUnix, locale) ?? '-'}</td>
                    <td className="track-col">{row.track ?? '-'}</td>
                    <td className="title-col">
                      <div className="title-cell">
                        <SongTitleButton
                          target={getSongDetailTarget(row)}
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
                        isHighlighted={row.isNewRecord}
                        variant="playlog"
                        onOpenHistory={canOpenHistory(row) ? () => onOpenHistory(row) : null}
                      />
                    </td>
                    <td className="rating-col">{formatNumber(toIntegerRating(row.rating), locale)}</td>
                    <td className="rank-col">{row.rank ?? '-'}</td>
                    <td className="fc-col">{row.fc ?? '-'}</td>
                    <td className="sync-col">{row.sync ?? '-'}</td>
                    <td className="dx-col">
                      {formatNumber(row.dxScore, locale)} / {formatNumber(row.dxScoreMax, locale)}
                    </td>
                  </tr>
                ))}
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
