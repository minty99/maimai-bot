import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent, type ReactNode } from 'react';

import {
  CHART_TYPES,
  DIFFICULTIES,
  DIFFICULTY_INDICES,
  DIFFICULTY_INDEX_LABELS,
  RANDOM_PICKER_DEFAULT_LEVEL,
  RANDOM_PICKER_FILTERS_STORAGE_KEY,
  RANDOM_PICKER_LEVEL_STEP,
  RANDOM_PICKER_MAX_LEVEL,
  RANDOM_PICKER_MIN_LEVEL,
  VERSION_ORDER_MAP,
} from '../app/constants';
import { useI18n, type TranslationKey } from '../app/i18n';
import {
  coerceArray,
  coerceNumber,
  coerceNumberArray,
  readStoredJson,
  type StoredRandomPickerFilters,
} from '../app/storage';
import { daysSince, parseMaimaiPlayedAtToUnix } from '../app/maimaiTime';
import { formatNumber, formatPercent, formatVersionLabel } from '../app/utils';
import { DEFAULT_SCORE_FILTERS } from '../app/scoreFilterPresets';
import { chartIdentityKey } from '../songIdentity';
import type {
  ChartType,
  DifficultyCategory,
  RandomPickerSong,
  ScoreApiResponse,
  SongInfoResponse,
  SongVersionResponse,
} from '../types';
import { getChartTypeToneClass } from './ChartTypeLabel';
import { DifficultyLabel, getDifficultyToneClass } from './DifficultyLabel';
import { Jacket } from './Jacket';

interface RandomPickerPageProps {
  sidebarTopContent?: ReactNode;
  songInfoUrl: string;
  scoreRecords: ScoreApiResponse[];
  songMetadata: Map<string, SongInfoResponse>;
  versionOptions: SongVersionResponse[];
}

type ModalKind = 'filters' | 'versions' | null;

interface FilterOption {
  value: string;
  label: ReactNode;
  subtitle?: string;
}

interface NumericRangeFilterCardProps {
  label: string;
  minValue: number;
  maxValue: number;
  onMinChange: (value: number) => void;
  onMaxChange: (value: number) => void;
  minInputStep?: number;
  maxInputStep?: number;
}

class PickerMessageError extends Error {
  constructor(
    message: string,
    readonly empty = false,
  ) {
    super(message);
    this.name = 'PickerMessageError';
  }
}

const DIFFICULTY_COLORS: Record<DifficultyCategory, string> = {
  BASIC: '#4f9d69',
  ADVANCED: '#dba631',
  EXPERT: '#d24e4e',
  MASTER: '#8455c6',
  'Re:MASTER': '#d96fa0',
};

function roundToStep(value: number): number {
  return Math.round(value * 10) / 10;
}

function clampLevel(value: number): number {
  return Math.min(Math.max(roundToStep(value), RANDOM_PICKER_MIN_LEVEL), RANDOM_PICKER_MAX_LEVEL);
}

function normalizeInput(value: string, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  return clampLevel(parsed);
}


function formatAchievement(achievementX10000: number | null): string {
  if (achievementX10000 === null) {
    return '--';
  }
  return formatPercent(achievementX10000 / 10000, 4);
}

function renderDifficultySummary(
  indices: number[],
  allLabel: string,
  includeLabel = true,
): ReactNode {
  if (indices.length === DIFFICULTIES.length) {
    return (
      <span className={includeLabel ? 'picker-summary-inline' : undefined}>
        {includeLabel ? <span>DIFF</span> : null}
        <span className="difficulty-summary-all">{allLabel}</span>
      </span>
    );
  }

  const values = [...indices]
    .sort((left, right) => left - right)
    .map((value) => DIFFICULTY_INDEX_LABELS[value]);

  return (
    <span className={includeLabel ? 'picker-summary-inline' : undefined}>
      {includeLabel ? <span>DIFF</span> : null}
      <span className="difficulty-summary-list">
        {values.map((value, index) => (
          <span key={value} className="difficulty-summary-item">
            <DifficultyLabel difficulty={value} short />
            {index < values.length - 1 ? <span className="difficulty-summary-separator">/</span> : null}
          </span>
        ))}
      </span>
    </span>
  );
}

function buildChartSummary(chartTypes: ChartType[], allLabel: string): string {
  if (chartTypes.length === CHART_TYPES.length) {
    return `TYPE ${allLabel}`;
  }
  return chartTypes.join('/');
}

function buildVersionSummary(
  includeVersionIndices: number[] | null,
  versionOptions: SongVersionResponse[],
  allLabel: string,
  versionsSelectedLabel: (count: string) => string,
  locale: string,
): string {
  if (includeVersionIndices === null) {
    return `VER ${allLabel}`;
  }

  const selectedCount = includeVersionIndices === null ? versionOptions.length : includeVersionIndices.length;
  return versionsSelectedLabel(selectedCount.toLocaleString(locale));
}

function buildCompactVersionSummary(
  includeVersionIndices: number[] | null,
  allLabel: string,
  locale: string,
): string {
  if (includeVersionIndices === null) {
    return `VER ${allLabel}`;
  }
  return `VER ${includeVersionIndices.length.toLocaleString(locale)}`;
}



function NumericRangeFilterCard({
  label,
  minValue,
  maxValue,
  onMinChange,
  onMaxChange,
  minInputStep = 1,
  maxInputStep = 1,
}: NumericRangeFilterCardProps) {
  return (
    <section className="picker-filter-section">
      <div className="filter-label">{label}</div>
      <div className="picker-range-row">
        <div className="picker-range-card">
          <span>MIN</span>
          <div className="picker-range-editor">
            <input
              type="number"
              inputMode="decimal"
              step={minInputStep}
              value={minValue}
              onChange={(event) => onMinChange(Number(event.target.value))}
            />
          </div>
        </div>
        <div className="picker-range-card">
          <span>MAX</span>
          <div className="picker-range-editor">
            <input
              type="number"
              inputMode="decimal"
              step={maxInputStep}
              value={maxValue}
              onChange={(event) => onMaxChange(Number(event.target.value))}
            />
          </div>
        </div>
      </div>
    </section>
  );
}

function getPickerResultMessage(error: Error, t: (key: TranslationKey) => string): { empty: boolean; message: string } {
  if (error instanceof PickerMessageError) {
    return {
      empty: error.empty,
      message: error.empty ? t('picker.noSongsHelp') : error.message,
    };
  }

  return {
    empty: false,
    message: error.message,
  };
}

function SelectionModal({
  title,
  options,
  selectedValues,
  onToggle,
  onSelectAll,
  onSelectNone,
  onClose,
}: {
  title: string;
  options: FilterOption[];
  selectedValues: string[];
  onToggle: (value: string) => void;
  onSelectAll: () => void;
  onSelectNone?: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal-card panel picker-filter-modal"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="detail-header">
          <h2>{title}</h2>
          <button type="button" className="modal-close-button" onClick={onClose}>{t('common.close')}</button>
        </div>
        <div className="picker-filter-toolbar">
          <button type="button" onClick={onSelectAll}>
            {t('picker.selectAll')}
          </button>
          {onSelectNone ? (
            <button type="button" onClick={onSelectNone}>
              {t('picker.clearAll')}
            </button>
          ) : null}
        </div>
        <div className="picker-filter-option-list">
          {options.map((option) => {
            const selected = selectedValues.includes(option.value);
            return (
              <button
                key={option.value}
                type="button"
                className={selected ? 'picker-filter-option active' : 'picker-filter-option'}
                onClick={() => onToggle(option.value)}
              >
                <span>{option.label}</span>
                {option.subtitle ? <small>{option.subtitle}</small> : null}
              </button>
            );
          })}
        </div>
      </section>
    </div>
  );
}

function FiltersMenu({
  chartTypes,
  difficultyIndices,
  versionSummary,
  isVersionLoading,
  versionError,
  achievementMin,
  achievementMax,
  daysMin,
  daysMax,
  onToggleChartType,
  onToggleDifficulty,
  onOpenVersions,
  onAchievementMinChange,
  onAchievementMaxChange,
  onDaysMinChange,
  onDaysMaxChange,
  onClose,
}: {
  chartTypes: ChartType[];
  difficultyIndices: number[];
  versionSummary: string;
  isVersionLoading: boolean;
  versionError: string | null;
  achievementMin: number;
  achievementMax: number;
  daysMin: number;
  daysMax: number;
  onToggleChartType: (value: ChartType) => void;
  onToggleDifficulty: (value: number) => void;
  onOpenVersions: () => void;
  onAchievementMinChange: (value: number) => void;
  onAchievementMaxChange: (value: number) => void;
  onDaysMinChange: (value: number) => void;
  onDaysMaxChange: (value: number) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal-card panel picker-filter-menu"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="detail-header">
          <h2>{t('picker.filterSettings')}</h2>
          <button type="button" className="modal-close-button" onClick={onClose}>
            {t('common.close')}
          </button>
        </div>
        <div className="picker-filter-sections">
          <section className="picker-filter-section">
            <div className="filter-label">Type</div>
            <div className="chip-row">
              {CHART_TYPES.map((value) => (
                <button
                  key={value}
                  type="button"
                  className={[
                    'chip',
                    chartTypes.includes(value) ? 'active' : '',
                    'chart-type-chip',
                    getChartTypeToneClass(value),
                  ]
                    .filter(Boolean)
                    .join(' ')}
                  onClick={() => onToggleChartType(value)}
                >
                  {value}
                </button>
              ))}
            </div>
          </section>

          <section className="picker-filter-section">
            <div className="filter-label">Diff</div>
            <div className="chip-row">
              {DIFFICULTY_INDICES.map((value) => {
                const difficulty = DIFFICULTY_INDEX_LABELS[value];
                return (
                  <button
                    key={value}
                    type="button"
                    className={[
                      'chip',
                      difficultyIndices.includes(value) ? 'active' : '',
                      'difficulty-chip',
                      getDifficultyToneClass(difficulty),
                    ]
                      .filter(Boolean)
                      .join(' ')}
                    onClick={() => onToggleDifficulty(value)}
                  >
                    <DifficultyLabel difficulty={difficulty} short />
                  </button>
                );
              })}
            </div>
          </section>

          <section className="picker-filter-section">
            <div className="filter-label">Ver</div>
            <button type="button" className="picker-version-launcher" onClick={onOpenVersions}>
              <strong>{isVersionLoading ? t('common.loadingVersions') : versionSummary}</strong>
              <span>{t('picker.chooseVersions')}</span>
            </button>
            {versionError ? <p className="picker-filter-error">{versionError}</p> : null}
          </section>

          <NumericRangeFilterCard
            label="Achievement"
            minValue={achievementMin}
            maxValue={achievementMax}
            minInputStep={0.0001}
            maxInputStep={0.0001}
            onMinChange={onAchievementMinChange}
            onMaxChange={onAchievementMaxChange}
          />

          <NumericRangeFilterCard
            label={t('picker.lastPlayedDays')}
            minValue={daysMin}
            maxValue={daysMax}
            onMinChange={onDaysMinChange}
            onMaxChange={onDaysMaxChange}
          />
        </div>
      </section>
    </div>
  );
}

export function RandomPickerPage({
  sidebarTopContent,
  songInfoUrl,
  scoreRecords,
  songMetadata,
  versionOptions,
}: RandomPickerPageProps) {
  const { formatNumber: formatLocalizedNumber, locale, t } = useI18n();
  const storedFilters = useMemo(
    () => readStoredJson<StoredRandomPickerFilters>(RANDOM_PICKER_FILTERS_STORAGE_KEY),
    [],
  );

  const initialFrom = useMemo(
    () => clampLevel(coerceNumber(storedFilters?.levelStart, RANDOM_PICKER_DEFAULT_LEVEL)),
    [storedFilters],
  );
  const initialTo = useMemo(() => {
    const fallbackTo = clampLevel(initialFrom);
    return clampLevel(coerceNumber(storedFilters?.levelEnd, fallbackTo));
  }, [initialFrom, storedFilters]);

  const [rangeFrom, setRangeFrom] = useState(Math.min(initialFrom, initialTo));
  const [rangeTo, setRangeTo] = useState(Math.max(initialFrom, initialTo));
  const [fromDraft, setFromDraft] = useState(() => Math.min(initialFrom, initialTo).toFixed(1));
  const [toDraft, setToDraft] = useState(() => Math.max(initialFrom, initialTo).toFixed(1));
  const [chartTypes, setChartTypes] = useState<ChartType[]>(() => {
    const values = coerceArray(storedFilters?.chartTypes, CHART_TYPES);
    return values.length > 0 ? values : [...CHART_TYPES];
  });
  const [difficultyIndices, setDifficultyIndices] = useState<number[]>(() => {
    const values = coerceNumberArray(storedFilters?.difficultyIndices)
      .filter((value) => DIFFICULTY_INDICES.includes(value as (typeof DIFFICULTY_INDICES)[number]))
      .sort((left, right) => left - right);
    return values.length > 0 ? values : [...DIFFICULTY_INDICES];
  });
  const [includeVersionIndices, setIncludeVersionIndices] = useState<number[] | null>(() => {
    if (!storedFilters || !('includeVersionIndices' in storedFilters)) {
      return null;
    }
    if (storedFilters.includeVersionIndices === null) {
      return null;
    }
    return coerceNumberArray(storedFilters.includeVersionIndices).sort((left, right) => left - right);
  });
  const [achievementMin, setAchievementMin] = useState(() =>
    coerceNumber(storedFilters?.achievementMin, DEFAULT_SCORE_FILTERS.achievementMin),
  );
  const [achievementMax, setAchievementMax] = useState(() =>
    coerceNumber(storedFilters?.achievementMax, DEFAULT_SCORE_FILTERS.achievementMax),
  );
  const [daysMin, setDaysMin] = useState(() =>
    coerceNumber(storedFilters?.daysMin, DEFAULT_SCORE_FILTERS.daysMin),
  );
  const [daysMax, setDaysMax] = useState(() =>
    coerceNumber(storedFilters?.daysMax, DEFAULT_SCORE_FILTERS.daysMax),
  );

  const [activeModal, setActiveModal] = useState<ModalKind>(null);
  const [pickedSong, setPickedSong] = useState<RandomPickerSong | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [pickerEmpty, setPickerEmpty] = useState(false);
  const [isPicking, setIsPicking] = useState(false);

  const pickAbortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const payload: StoredRandomPickerFilters = {
      levelStart: rangeFrom,
      levelEnd: rangeTo,
      chartTypes,
      difficultyIndices,
      includeVersionIndices,
      achievementMin,
      achievementMax,
      daysMin,
      daysMax,
    };
    localStorage.setItem(RANDOM_PICKER_FILTERS_STORAGE_KEY, JSON.stringify(payload));
  }, [
    achievementMax,
    achievementMin,
    chartTypes,
    daysMax,
    daysMin,
    difficultyIndices,
    includeVersionIndices,
    rangeFrom,
    rangeTo,
  ]);

  const sortedVersionOptions = useMemo(() => {
    return [...versionOptions].sort((left, right) => {
      const leftOrder = VERSION_ORDER_MAP.get(left.version_name) ?? Number.MAX_SAFE_INTEGER;
      const rightOrder = VERSION_ORDER_MAP.get(right.version_name) ?? Number.MAX_SAFE_INTEGER;
      if (leftOrder !== rightOrder) {
        return leftOrder - rightOrder;
      }
      return left.version_index - right.version_index;
    });
  }, [versionOptions]);

  useEffect(() => {
    if (includeVersionIndices === null || versionOptions.length === 0) {
      return;
    }
    const versionSet = new Set(versionOptions.map((option) => option.version_index));
    const next = includeVersionIndices.filter((value) => versionSet.has(value));
    if (next.length !== includeVersionIndices.length) {
      setIncludeVersionIndices(next);
    }
  }, [includeVersionIndices, versionOptions]);

  useEffect(() => {
    return () => {
      pickAbortRef.current?.abort();
    };
  }, []);

  const scoreMap = useMemo(() => {
    const next = new Map<string, ScoreApiResponse>();
    for (const score of scoreRecords) {
      next.set(
        chartIdentityKey(
          score.title,
          score.genre,
          score.artist,
          score.chart_type,
          score.diff_category,
        ),
        score,
      );
    }
    return next;
  }, [scoreRecords]);

  const syncDrafts = useCallback((nextFrom: number, nextTo: number) => {
    setFromDraft(nextFrom.toFixed(1));
    setToDraft(nextTo.toFixed(1));
  }, []);

  const applyRange = useCallback((nextFrom: number, nextTo: number) => {
    const normalizedFrom = clampLevel(Math.min(nextFrom, nextTo));
    const normalizedTo = clampLevel(Math.max(nextFrom, nextTo));
    setRangeFrom(normalizedFrom);
    setRangeTo(normalizedTo);
    syncDrafts(normalizedFrom, normalizedTo);
  }, [syncDrafts]);

  const nudgeFrom = useCallback((delta: number) => {
    const nextFrom = clampLevel(rangeFrom + delta);
    applyRange(nextFrom, rangeTo);
  }, [applyRange, rangeFrom, rangeTo]);

  const nudgeTo = useCallback((delta: number) => {
    const nextTo = clampLevel(rangeTo + delta);
    applyRange(rangeFrom, nextTo);
  }, [applyRange, rangeFrom, rangeTo]);

  const handleFromInput = useCallback((event: ChangeEvent<HTMLInputElement>) => {
    setFromDraft(event.target.value);
  }, []);

  const handleToInput = useCallback((event: ChangeEvent<HTMLInputElement>) => {
    setToDraft(event.target.value);
  }, []);

  const commitFrom = useCallback(() => {
    const nextFrom = normalizeInput(fromDraft, rangeFrom);
    applyRange(nextFrom, rangeTo);
  }, [applyRange, fromDraft, rangeFrom, rangeTo]);

  const commitTo = useCallback(() => {
    const nextTo = normalizeInput(toDraft, rangeTo);
    applyRange(rangeFrom, nextTo);
  }, [applyRange, rangeFrom, rangeTo, toDraft]);



  const normalizedAchievementMin = Math.min(achievementMin, achievementMax);
  const normalizedAchievementMax = Math.max(achievementMin, achievementMax);
  const normalizedDaysMin = Math.max(0, Math.min(daysMin, daysMax));
  const normalizedDaysMax = Math.max(0, Math.max(daysMin, daysMax));

  const toggleChartType = useCallback((value: ChartType) => {
    setChartTypes((current) => {
      if (current.includes(value)) {
        return current.length === 1 ? current : current.filter((item) => item !== value);
      }
      return [...current, value].sort((left, right) => CHART_TYPES.indexOf(left) - CHART_TYPES.indexOf(right));
    });
  }, []);

  const toggleDifficulty = useCallback((value: number) => {
    setDifficultyIndices((current) => {
      if (current.includes(value)) {
        return current.length === 1 ? current : current.filter((item) => item !== value);
      }
      return [...current, value].sort((left, right) => left - right);
    });
  }, []);

  const toggleVersion = useCallback((value: number) => {
    setIncludeVersionIndices((current) => {
      const base = current === null ? versionOptions.map((option) => option.version_index) : current;
      if (base.includes(value)) {
        return base.filter((item) => item !== value);
      }
      const next = [...base, value].sort((left, right) => left - right);
      if (versionOptions.length > 0 && next.length === versionOptions.length) {
        return null;
      }
      return next;
    });
  }, [versionOptions]);

  const handlePickRandom = useCallback(async () => {
    pickAbortRef.current?.abort();
    const controller = new AbortController();
    pickAbortRef.current = controller;

    setIsPicking(true);
    setPickedSong(null);
    setPickerError(null);
    setPickerEmpty(false);

    try {
      if (!songInfoUrl.trim()) {
        throw new PickerMessageError(t('picker.songInfoRequired'));
      }

      const chartTypeSet = new Set(chartTypes);
      const difficultySet = new Set(difficultyIndices.map((index) => DIFFICULTY_INDEX_LABELS[index]));
      const versionSet = includeVersionIndices === null ? null : new Set(includeVersionIndices);

      const candidates: RandomPickerSong[] = [];

      for (const song of songMetadata.values()) {
        for (const sheet of song.sheets) {
          if (!sheet.region.intl) {
            continue;
          }
          if (sheet.internal_level === null || sheet.internal_level < rangeFrom || sheet.internal_level > rangeTo) {
            continue;
          }

          const key = chartIdentityKey(
            song.title,
            song.genre,
            song.artist,
            sheet.chart_type,
            sheet.difficulty,
          );
          if (!chartTypeSet.has(sheet.chart_type)) {
            continue;
          }
          if (!difficultySet.has(sheet.difficulty)) {
            continue;
          }

          const versionIndex = sortedVersionOptions.find(
            (option) => option.version_name === (sheet.version ?? null),
          )?.version_index;
          if (versionSet && (versionIndex === undefined || !versionSet.has(versionIndex))) {
            continue;
          }

          const score = scoreMap.get(key);
          const achievementPercent = (score?.achievement_x10000 ?? 0) / 10000;
          if (achievementPercent < normalizedAchievementMin || achievementPercent > normalizedAchievementMax) {
            continue;
          }

          const latestPlayedAtUnix = parseMaimaiPlayedAtToUnix(score?.last_played_at);
          const daysSinceLastPlayed = daysSince(latestPlayedAtUnix);
          if (
            daysSinceLastPlayed !== null &&
            (daysSinceLastPlayed < normalizedDaysMin || daysSinceLastPlayed > normalizedDaysMax)
          ) {
            continue;
          }

          candidates.push({
            title: song.title,
            genre: song.genre,
            artist: song.artist,
            version: sheet.version,
            imageName: song.image_name,
            chartType: sheet.chart_type,
            difficulty: sheet.difficulty,
            level: sheet.level,
            internalLevel: sheet.internal_level,
            achievementX10000: score?.achievement_x10000 ?? null,
            rank: score?.rank ?? null,
            fc: score?.fc ?? null,
            sync: score?.sync ?? null,
            dxScore: score?.dx_score ?? null,
            dxScoreMax: score?.dx_score_max ?? null,
            lastPlayedAt: score?.last_played_at ?? null,
            playCount: typeof score?.play_count === 'number' ? Math.trunc(score.play_count) : null,
            levelSongCount: null,
            filteredSongCount: null,
          });
        }
      }

      if (candidates.length === 0) {
        throw new PickerMessageError(t('picker.noSongs'), true);
      }

      const selected = candidates[Math.floor(Math.random() * candidates.length)];
      const withStats: RandomPickerSong = {
        ...selected,
        filteredSongCount: candidates.length,
        levelSongCount: candidates.length,
      };

      if (!controller.signal.aborted) {
        setPickedSong(withStats);
      }
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }
      const nextError = error instanceof Error ? error : new Error(String(error));
      const result = getPickerResultMessage(nextError, t);
      setPickerEmpty(result.empty);
      setPickerError(result.empty ? null : result.message);
    } finally {
      if (!controller.signal.aborted) {
        setIsPicking(false);
      }
    }
  }, [
    chartTypes,
    difficultyIndices,
    includeVersionIndices,
    normalizedAchievementMax,
    normalizedAchievementMin,
    normalizedDaysMax,
    normalizedDaysMin,
    rangeFrom,
    rangeTo,
    scoreMap,
    songInfoUrl,
    songMetadata,
    sortedVersionOptions,
  ]);

  const chartSummary = useMemo(
    () => buildChartSummary(chartTypes, t('common.all')),
    [chartTypes, t],
  );
  const difficultySummaryNode = useMemo(
    () => renderDifficultySummary(difficultyIndices, t('common.all')),
    [difficultyIndices, t],
  );
  const versionSummary = useMemo(
    () => buildVersionSummary(
      includeVersionIndices,
      sortedVersionOptions,
      t('common.all'),
      (count) => t('picker.versionsSelected', { count }),
      locale,
    ),
    [includeVersionIndices, locale, sortedVersionOptions, t],
  );
  const compactVersionSummary = useMemo(
    () => buildCompactVersionSummary(includeVersionIndices, t('common.all'), locale),
    [includeVersionIndices, locale, t],
  );
  const versionModalOptions = useMemo<FilterOption[]>(
    () =>
      sortedVersionOptions.map((option) => ({
        value: String(option.version_index),
        label: formatVersionLabel(option.version_name),
        subtitle: t('units.songs', { count: option.song_count.toLocaleString(locale) }),
      })),
    [locale, sortedVersionOptions, t],
  );
  const selectedVersionValues = useMemo(() => {
    if (includeVersionIndices === null) {
      return sortedVersionOptions.map((option) => String(option.version_index));
    }
    return includeVersionIndices.map(String);
  }, [includeVersionIndices, sortedVersionOptions]);

  const renderStateCard = useCallback((title: string, tone?: 'error') => {
    const stateClassName = tone === 'error'
      ? 'picker-song-card picker-song-card--placeholder error'
      : 'picker-song-card picker-song-card--placeholder';

    return (
      <article className={stateClassName}>
        <div className="picker-song-stage picker-song-stage--placeholder">
          <div className="picker-stage-gradient" />
          <div className="picker-stage-badges">
            <span className="picker-badge difficulty">DIFF</span>
            <span className="picker-badge">TYPE</span>
            <span className="picker-badge muted">VER</span>
          </div>
          <div className="picker-stage-placeholder">
            <h3>{title}</h3>
          </div>
        </div>
        <div className="picker-song-info picker-song-info--placeholder">
          <div className="picker-skeleton picker-skeleton-title" />
          <div className="picker-skeleton picker-skeleton-subtitle" />
          <div className="picker-skeleton picker-skeleton-pool" />
          <div className="picker-skeleton picker-skeleton-meta" />
          <div className="picker-skeleton picker-skeleton-stats" />
        </div>
      </article>
    );
  }, []);

  const resultView = (() => {
    if (isPicking) {
      return renderStateCard(t('picker.picking'));
    }

    if (pickerError) {
      return renderStateCard(t('picker.pickFailed'), 'error');
    }

    if (pickerEmpty) {
      return renderStateCard(t('picker.noSongs'));
    }

    if (!pickedSong) {
      return renderStateCard(t('picker.random'));
    }

    const difficultyColor = DIFFICULTY_COLORS[pickedSong.difficulty];
    const hasPersonal =
      pickedSong.achievementX10000 !== null ||
      pickedSong.fc !== null ||
      pickedSong.sync !== null ||
      pickedSong.lastPlayedAt !== null ||
      pickedSong.playCount !== null;
    const achievementLabel = hasPersonal
      ? formatAchievement(pickedSong.achievementX10000)
      : t('picker.noData');
    const rankLabel = hasPersonal
      ? (pickedSong.rank ?? t('picker.unplayed'))
      : t('picker.unplayed');
    const fcLabel = hasPersonal
      ? (pickedSong.fc ?? t('picker.fcEmpty'))
      : t('picker.fcEmpty');
    const syncLabel = hasPersonal
      ? (pickedSong.sync ?? t('picker.syncEmpty'))
      : t('picker.syncEmpty');
    const metaLabel = hasPersonal
      ? t('picker.metaWithData', {
        lastPlayed: pickedSong.lastPlayedAt ?? t('picker.noData'),
        playCount: pickedSong.playCount !== null ? formatLocalizedNumber(pickedSong.playCount) : t('picker.noData'),
      })
      : t('picker.metaNoData');

    return (
      <article className="picker-song-card" style={{ ['--picker-accent' as string]: difficultyColor }}>
        <div className="picker-song-stage">
          <Jacket
            songInfoUrl={songInfoUrl}
            imageName={pickedSong.imageName}
            title={pickedSong.title}
            className="picker-jacket"
          />
          <div className="picker-stage-gradient" />
          <div className="picker-stage-badges">
            <span className="picker-badge difficulty">
              <DifficultyLabel difficulty={pickedSong.difficulty} />
            </span>
            <span className="picker-badge">{pickedSong.chartType}</span>
            {pickedSong.version ? <span className="picker-badge muted">{pickedSong.version}</span> : null}
          </div>
        </div>

        <div className="picker-song-info">
          <h3>{pickedSong.title}</h3>
          <p className={`picker-level-line ${getDifficultyToneClass(pickedSong.difficulty)}`}>
            Lv {pickedSong.level}
            {pickedSong.internalLevel !== null ? ` (${pickedSong.internalLevel.toFixed(1)})` : ''}
          </p>

          {pickedSong.filteredSongCount !== null && pickedSong.levelSongCount !== null ? (
            <p className="picker-pool-line">
              {t('picker.pickedFrom', { count: formatNumber(pickedSong.filteredSongCount, locale) })}
            </p>
          ) : (
            <p className="picker-pool-line">{t('picker.pickedFromNoData')}</p>
          )}

          <div className={hasPersonal ? 'picker-achievement-row' : 'picker-achievement-row is-empty'}>
            <strong>{achievementLabel}</strong>
            <span>{rankLabel}</span>
            <div className="picker-tag-row">
              <em>{fcLabel}</em>
              <em>{syncLabel}</em>
            </div>
          </div>

          <p className={hasPersonal ? 'picker-meta-line' : 'picker-meta-line is-empty'}>
            {metaLabel}
          </p>
        </div>
      </article>
    );
  })();

  const modal = (() => {
    if (activeModal === 'filters') {
      return (
        <FiltersMenu
          chartTypes={chartTypes}
          difficultyIndices={difficultyIndices}
          versionSummary={versionSummary}
          isVersionLoading={false}
          versionError={null}
          onToggleChartType={toggleChartType}
          onToggleDifficulty={toggleDifficulty}
          onOpenVersions={() => setActiveModal('versions')}
          achievementMin={achievementMin}
          achievementMax={achievementMax}
          daysMin={daysMin}
          daysMax={daysMax}
          onAchievementMinChange={setAchievementMin}
          onAchievementMaxChange={setAchievementMax}
          onDaysMinChange={setDaysMin}
          onDaysMaxChange={setDaysMax}
          onClose={() => setActiveModal(null)}
        />
      );
    }

    if (activeModal === 'versions') {
      return (
        <SelectionModal
          title={t('common.version')}
          options={versionModalOptions}
          selectedValues={selectedVersionValues}
          onToggle={(value) => toggleVersion(Number(value))}
          onSelectAll={() => setIncludeVersionIndices(null)}
          onSelectNone={() => setIncludeVersionIndices([])}
          onClose={() => setActiveModal('filters')}
        />
      );
    }

    return null;
  })();

  const pickerStatusLabel = isPicking
    ? t('picker.statusPicking')
    : pickerError
      ? t('picker.statusError')
      : pickerEmpty
        ? t('picker.statusEmpty')
        : pickedSong
          ? t('picker.statusPicked')
          : t('picker.statusReady');

  return (
    <>
      <section className="picker-layout">
        <div className="picker-control-column">
          {sidebarTopContent}
          <section className="panel picker-control-panel">
            <div className="panel-heading">
              <div>
                <h2>{t('picker.controlTitle')}</h2>
              </div>
              <button
                type="button"
                className="picker-filter-launcher"
                onClick={() => setActiveModal('filters')}
              >
                {t('picker.filters')}
              </button>
            </div>

            <div className="picker-summary-row">
             <span className="toolbar-pill">LV {rangeFrom.toFixed(1)} - {rangeTo.toFixed(1)}</span>
              <span className="toolbar-pill">{chartSummary}</span>
              <span className="toolbar-pill">{difficultySummaryNode}</span>
              <span className="toolbar-pill">{compactVersionSummary}</span>
            </div>

            <div className="picker-range-row">
              <div className="picker-range-card">
                <span>FROM</span>
                <div className="picker-range-editor">
                  <button type="button" onClick={() => nudgeFrom(-RANDOM_PICKER_LEVEL_STEP)}>
                    -
                  </button>
                  <input
                    inputMode="decimal"
                    value={fromDraft}
                    onChange={handleFromInput}
                    onBlur={commitFrom}
                  />
                  <button type="button" onClick={() => nudgeFrom(RANDOM_PICKER_LEVEL_STEP)}>
                    +
                  </button>
                </div>
              </div>
              <div className="picker-range-card">
                <span>TO</span>
                <div className="picker-range-editor">
                  <button type="button" onClick={() => nudgeTo(-RANDOM_PICKER_LEVEL_STEP)}>
                    -
                  </button>
                  <input
                    inputMode="decimal"
                    value={toDraft}
                    onChange={handleToInput}
                    onBlur={commitTo}
                  />
                  <button type="button" onClick={() => nudgeTo(RANDOM_PICKER_LEVEL_STEP)}>
                    +
                  </button>
                </div>
              </div>
            </div>

          </section>

          <button type="button" className="picker-random-button" onClick={() => void handlePickRandom()}>
            {isPicking ? t('picker.picking').toUpperCase() : t('picker.random')}
          </button>
        </div>

        <section className="panel picker-result-panel">
          <div className="panel-heading">
            <div>
              <h2>{t('picker.selection')}</h2>
            </div>
            <span className="panel-count">{pickerStatusLabel}</span>
          </div>

          <div className="picker-result-stage">{resultView}</div>
        </section>
      </section>

      {modal}
    </>
  );
}
