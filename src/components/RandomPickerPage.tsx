import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react';

import { fetchRandomPickerSong, fetchSongVersions } from '../api';
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
import {
  coerceArray,
  coerceNumber,
  coerceNumberArray,
  readStoredJson,
  type StoredRandomPickerFilters,
} from '../app/storage';
import { formatDifficultyShort, formatNumber, formatPercent } from '../app/utils';
import type { ChartType, DifficultyCategory, RandomPickerSong, SongVersionResponse } from '../types';
import { Jacket } from './Jacket';

interface RandomPickerPageProps {
  songInfoUrl: string;
  recordCollectorUrl: string;
}

type ModalKind = 'filters' | 'chartTypes' | 'difficulties' | 'versions' | null;

interface FilterOption {
  value: string;
  label: string;
  subtitle?: string;
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

function buildDifficultySummary(indices: number[]): string {
  if (indices.length === DIFFICULTIES.length) {
    return 'DIFF ALL';
  }
  const labels = indices
    .map((value) => formatDifficultyShort(DIFFICULTY_INDEX_LABELS[value]))
    .sort((left, right) => left.localeCompare(right, 'ko'));
  return `DIFF ${labels.join('/')}`;
}

function buildChartSummary(chartTypes: ChartType[]): string {
  if (chartTypes.length === CHART_TYPES.length) {
    return 'TYPE ALL';
  }
  return chartTypes.join('/');
}

function buildVersionSummary(
  includeVersionIndices: number[] | null,
  versionOptions: SongVersionResponse[],
): string {
  if (includeVersionIndices === null) {
    return 'VER ALL';
  }
  if (includeVersionIndices.length === 0) {
    return 'VER NONE';
  }
  if (versionOptions.length > 0 && includeVersionIndices.length === versionOptions.length) {
    return 'VER ALL';
  }
  return `VER ${includeVersionIndices.length}`;
}

function getPickerResultMessage(error: Error): { empty: boolean; message: string } {
  if (error.message.includes('HTTP 404')) {
    return {
      empty: true,
      message: '조건에 맞는 곡이 없습니다. 범위를 넓히거나 필터를 완화해보세요.',
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
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal-card panel picker-filter-modal"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="detail-header">
          <h2>{title}</h2>
          <button type="button" onClick={onClose}>
            닫기
          </button>
        </div>
        <div className="picker-filter-toolbar">
          <button type="button" onClick={onSelectAll}>
            전체 선택
          </button>
          {onSelectNone ? (
            <button type="button" onClick={onSelectNone}>
              전체 해제
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
  chartSummary,
  difficultySummary,
  versionSummary,
  isVersionLoading,
  onOpenChartTypes,
  onOpenDifficulties,
  onOpenVersions,
  onClose,
}: {
  chartSummary: string;
  difficultySummary: string;
  versionSummary: string;
  isVersionLoading: boolean;
  onOpenChartTypes: () => void;
  onOpenDifficulties: () => void;
  onOpenVersions: () => void;
  onClose: () => void;
}) {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal-card panel picker-filter-menu"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="detail-header">
          <h2>Filter Settings</h2>
          <button type="button" onClick={onClose}>
            닫기
          </button>
        </div>
        <div className="picker-filter-menu-grid">
          <button type="button" onClick={onOpenChartTypes}>
            <span>TYPE</span>
            <strong>{chartSummary}</strong>
          </button>
          <button type="button" onClick={onOpenDifficulties}>
            <span>DIFF</span>
            <strong>{difficultySummary}</strong>
          </button>
          <button type="button" onClick={onOpenVersions}>
            <span>VER</span>
            <strong>{isVersionLoading ? '...' : versionSummary}</strong>
          </button>
        </div>
      </section>
    </div>
  );
}

export function RandomPickerPage({ songInfoUrl, recordCollectorUrl }: RandomPickerPageProps) {
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
  const [versionOptions, setVersionOptions] = useState<SongVersionResponse[]>([]);
  const [isLoadingVersions, setIsLoadingVersions] = useState(false);
  const [versionError, setVersionError] = useState<string | null>(null);
  const [activeModal, setActiveModal] = useState<ModalKind>(null);
  const [pickedSong, setPickedSong] = useState<RandomPickerSong | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [pickerEmpty, setPickerEmpty] = useState(false);
  const [isPicking, setIsPicking] = useState(false);

  const pickAbortRef = useRef<AbortController | null>(null);
  const versionAbortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const payload: StoredRandomPickerFilters = {
      levelStart: rangeFrom,
      levelEnd: rangeTo,
      chartTypes,
      difficultyIndices,
      includeVersionIndices,
    };
    localStorage.setItem(RANDOM_PICKER_FILTERS_STORAGE_KEY, JSON.stringify(payload));
  }, [chartTypes, difficultyIndices, includeVersionIndices, rangeFrom, rangeTo]);

  useEffect(() => {
    versionAbortRef.current?.abort();

    if (!songInfoUrl.trim()) {
      setVersionOptions([]);
      setVersionError('Song Info URL을 먼저 입력해야 버전 필터를 불러올 수 있습니다.');
      setIsLoadingVersions(false);
      return;
    }

    const controller = new AbortController();
    versionAbortRef.current = controller;
    setIsLoadingVersions(true);
    setVersionError(null);

    void fetchSongVersions(songInfoUrl, controller.signal)
      .then((versions) => {
        if (controller.signal.aborted) {
          return;
        }
        const sortedVersions = [...versions].sort((left, right) => {
          const leftOrder = VERSION_ORDER_MAP.get(left.version_name) ?? Number.MAX_SAFE_INTEGER;
          const rightOrder = VERSION_ORDER_MAP.get(right.version_name) ?? Number.MAX_SAFE_INTEGER;
          if (leftOrder !== rightOrder) {
            return leftOrder - rightOrder;
          }
          return left.version_index - right.version_index;
        });
        setVersionOptions(sortedVersions);
      })
      .catch((error) => {
        if (controller.signal.aborted) {
          return;
        }
        const message = error instanceof Error ? error.message : String(error);
        setVersionOptions([]);
        setVersionError(`버전 목록을 불러오지 못했습니다: ${message}`);
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoadingVersions(false);
        }
      });

    return () => controller.abort();
  }, [songInfoUrl]);

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
      versionAbortRef.current?.abort();
    };
  }, []);

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
      const song = await fetchRandomPickerSong({
        songInfoBaseUrl: songInfoUrl,
        recordCollectorBaseUrl: recordCollectorUrl,
        minLevel: rangeFrom,
        maxLevel: rangeTo,
        chartTypes,
        difficultyIndices,
        includeVersionIndices,
        signal: controller.signal,
      });

      if (!controller.signal.aborted) {
        setPickedSong(song);
      }
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }
      const nextError = error instanceof Error ? error : new Error(String(error));
      const result = getPickerResultMessage(nextError);
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
    rangeFrom,
    rangeTo,
    recordCollectorUrl,
    songInfoUrl,
  ]);

  const chartSummary = useMemo(() => buildChartSummary(chartTypes), [chartTypes]);
  const difficultySummary = useMemo(
    () => buildDifficultySummary(difficultyIndices),
    [difficultyIndices],
  );
  const versionSummary = useMemo(
    () => buildVersionSummary(includeVersionIndices, versionOptions),
    [includeVersionIndices, versionOptions],
  );

  const chartOptions = useMemo<FilterOption[]>(
    () => CHART_TYPES.map((value) => ({ value, label: value })),
    [],
  );
  const difficultyOptions = useMemo<FilterOption[]>(
    () =>
      DIFFICULTY_INDICES.map((value) => ({
        value: String(value),
        label: DIFFICULTY_INDEX_LABELS[value],
      })),
    [],
  );
  const versionModalOptions = useMemo<FilterOption[]>(
    () =>
      versionOptions.map((option) => ({
        value: String(option.version_index),
        label: option.version_name,
        subtitle: `${option.song_count.toLocaleString()} songs`,
      })),
    [versionOptions],
  );
  const selectedVersionValues = useMemo(() => {
    if (includeVersionIndices === null) {
      return versionOptions.map((option) => String(option.version_index));
    }
    return includeVersionIndices.map(String);
  }, [includeVersionIndices, versionOptions]);

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
      return renderStateCard('선곡 중...');
    }

    if (pickerError) {
      return renderStateCard('랜덤 선곡 실패', 'error');
    }

    if (pickerEmpty) {
      return renderStateCard('조건에 맞는 곡이 없습니다');
    }

    if (!pickedSong) {
      return renderStateCard('RANDOM');
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
      : 'No Data';
    const rankLabel = hasPersonal
      ? (pickedSong.rank ?? 'UNPLAYED')
      : 'UNPLAYED';
    const fcLabel = hasPersonal
      ? (pickedSong.fc ?? 'FC -')
      : 'FC -';
    const syncLabel = hasPersonal
      ? (pickedSong.sync ?? 'SYNC -')
      : 'SYNC -';
    const metaLabel = hasPersonal
      ? `${pickedSong.lastPlayedAt ? `Last played: ${pickedSong.lastPlayedAt}` : 'Last played: No Data'}  •  ${pickedSong.playCount !== null ? `Play count: ${pickedSong.playCount}` : 'Play count: No Data'}`
      : 'Last played: No Data  •  Play count: No Data';

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
            <span className="picker-badge difficulty">{pickedSong.difficulty}</span>
            <span className="picker-badge">{pickedSong.chartType}</span>
            {pickedSong.version ? <span className="picker-badge muted">{pickedSong.version}</span> : null}
          </div>
        </div>

        <div className="picker-song-info">
          <h3>{pickedSong.title}</h3>
          <p className="picker-level-line">
            {pickedSong.level}
            {pickedSong.internalLevel !== null ? ` (${pickedSong.internalLevel.toFixed(1)}` : ''}
            {pickedSong.internalLevel !== null && pickedSong.userLevel ? ` / ${pickedSong.userLevel}` : ''}
            {pickedSong.internalLevel !== null ? ')' : pickedSong.userLevel ? ` (${pickedSong.userLevel})` : ''}
          </p>

          {pickedSong.filteredSongCount !== null && pickedSong.levelSongCount !== null ? (
            <p className="picker-pool-line">
              Picked from {formatNumber(pickedSong.filteredSongCount)} / {formatNumber(pickedSong.levelSongCount)} songs
            </p>
          ) : (
            <p className="picker-pool-line">Picked from No Data / No Data songs</p>
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
          chartSummary={chartSummary}
          difficultySummary={difficultySummary}
          versionSummary={versionSummary}
          isVersionLoading={isLoadingVersions}
          onOpenChartTypes={() => setActiveModal('chartTypes')}
          onOpenDifficulties={() => setActiveModal('difficulties')}
          onOpenVersions={() => setActiveModal('versions')}
          onClose={() => setActiveModal(null)}
        />
      );
    }

    if (activeModal === 'chartTypes') {
      return (
        <SelectionModal
          title="Chart Type"
          options={chartOptions}
          selectedValues={chartTypes}
          onToggle={(value) => toggleChartType(value as ChartType)}
          onSelectAll={() => setChartTypes([...CHART_TYPES])}
          onClose={() => setActiveModal('filters')}
        />
      );
    }

    if (activeModal === 'difficulties') {
      return (
        <SelectionModal
          title="Difficulty"
          options={difficultyOptions}
          selectedValues={difficultyIndices.map(String)}
          onToggle={(value) => toggleDifficulty(Number(value))}
          onSelectAll={() => setDifficultyIndices([...DIFFICULTY_INDICES])}
          onClose={() => setActiveModal('filters')}
        />
      );
    }

    if (activeModal === 'versions') {
      return (
        <SelectionModal
          title="Versions"
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

  return (
    <>
      <section className="picker-layout compact">
        <section className="panel picker-control-panel compact">
          <div className="picker-toolbar">
            <h2>Random Picker</h2>
            <button
              type="button"
              className="picker-filter-launcher"
              onClick={() => setActiveModal('filters')}
            >
              설정
            </button>
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

          {versionError ? <p className="muted">{versionError}</p> : null}
        </section>

        <button type="button" className="picker-random-button" onClick={() => void handlePickRandom()}>
          {isPicking ? 'PICKING...' : 'RANDOM'}
        </button>

        <section className="panel picker-result-panel compact">{resultView}</section>
      </section>

      {modal}
    </>
  );
}
