import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import {
  fetchExplorerPayload,
} from './api';
import {
  AAA_OR_BELOW_RANKS,
  CHART_TYPES,
  DEFAULT_RECORD_COLLECTOR_URL,
  DEFAULT_SONG_INFO_URL,
  DIFFICULTIES,
  FC_ORDER,
  PLAYLOG_FILTERS_STORAGE_KEY,
  PlaylogSortKey,
  RECORD_STORAGE_KEY,
  RANK_FILTER_AAA_OR_BELOW_LABEL,
  SCORE_FILTERS_STORAGE_KEY,
  SCORE_RANK_ORDER,
  SCORE_RANK_ORDER_MAP,
  SONG_INFO_STORAGE_KEY,
  TABLE_LAYOUT_STORAGE_KEY,
  ScoreSortKey,
  SYNC_ORDER,
  VERSION_ORDER_MAP,
} from './app/constants';
import { sortByOrder, toggleArrayValue } from './app/utils';
import {
  coerceArray,
  coerceNumber,
  coerceStringArray,
  readStoredJson,
  readStoredValue,
  StoredPlaylogFilters,
  StoredScoreFilters,
} from './app/storage';
import {
  buildPlaylogRows,
  buildScoreHistoryPoints,
  buildSongDetailRows,
  buildScoreRows,
} from './derive';
import {
  buildFilteredPlaylogRows,
  buildFilteredScoreRows,
  computeFcOptions,
  computeScoreRankOptions,
  computeSyncOptions,
} from './app/filtering';
import { PlaylogExplorerSection } from './components/PlaylogExplorerSection';
import { RandomPickerPage } from './components/RandomPickerPage';
import { RatingPage } from './components/RatingPage';
import { ScoreExplorerSection } from './components/ScoreExplorerSection';
import { SettingsPage } from './components/SettingsPage';
import { SongDetailModal } from './components/SongDetailModal';
import { ScoreHistoryModal } from './components/ScoreHistoryModal';
import { songIdentityKey } from './songIdentity';
import type {
  ChartType,
  DifficultyCategory,
  FcStatus,
  PlayRecordApiResponse,
  PlaylogRow,
  ScoreApiResponse,
  ScoreRow,
  ScoreRank,
  SongInfoResponse,
  SongVersionResponse,
  SyncStatus,
} from './types';

type AppPage = 'scores' | 'rating' | 'playlogs' | 'picker' | 'settings';

function readPageFromHash(hash: string): AppPage {
  if (hash === '#rating') {
    return 'rating';
  }
  if (hash === '#playlogs') {
    return 'playlogs';
  }
  if (hash === '#picker') {
    return 'picker';
  }
  if (hash === '#settings') {
    return 'settings';
  }
  return 'scores';
}

function readShowJacketsPreference(): boolean {
  return localStorage.getItem(TABLE_LAYOUT_STORAGE_KEY) !== 'compact';
}

const MAIMAI_DAY_START_HOUR = 4;
const MAIMAI_DAY_OFFSET_SECONDS = MAIMAI_DAY_START_HOUR * 60 * 60;

function toMaimaiDayKey(playedAtUnix: number): string {
  const date = new Date((playedAtUnix - MAIMAI_DAY_OFFSET_SECONDS) * 1000);
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, '0');
  const day = `${date.getDate()}`.padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function toMaimaiDayStartUnix(dayKey: string): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dayKey);
  if (!match) {
    return null;
  }

  const [, yearText, monthText, dayText] = match;
  const start = new Date(
    Number(yearText),
    Number(monthText) - 1,
    Number(dayText),
    MAIMAI_DAY_START_HOUR,
    0,
    0,
    0,
  );
  const unix = Math.floor(start.getTime() / 1000);
  return Number.isFinite(unix) ? unix : null;
}

function countCredits(rows: PlaylogRow[]): number | null {
  const creditIds = rows
    .map((row) => row.creditId)
    .filter((creditId): creditId is number => creditId !== null);

  if (creditIds.length === 0) {
    return null;
  }

  return new Set(creditIds).size;
}

function ScoresIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 19V5" />
      <path d="M8 19V11" />
      <path d="M12 19V8" />
      <path d="M16 19V13" />
      <path d="M20 19V6" />
    </svg>
  );
}

function PlaylogsIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 6h16" />
      <path d="M4 12h16" />
      <path d="M4 18h16" />
      <circle cx="7" cy="6" r="1" fill="currentColor" stroke="none" />
      <circle cx="7" cy="12" r="1" fill="currentColor" stroke="none" />
      <circle cx="7" cy="18" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

function RatingIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 20h16" />
      <path d="M7 16 10.2 9.8 13.4 14.2 17 7" />
      <circle cx="17" cy="7" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

function PickerIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 4h12" />
      <path d="M9 4v4" />
      <path d="M15 4v4" />
      <path d="M12 8v4" />
      <path d="M7 20c0-2.8 2.2-5 5-5s5 2.2 5 5" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1 1 0 0 0 .2 1.1l.1.1a2 2 0 0 1-2.8 2.8l-.1-.1a1 1 0 0 0-1.1-.2 1 1 0 0 0-.6.9V20a2 2 0 0 1-4 0v-.2a1 1 0 0 0-.6-.9 1 1 0 0 0-1.1.2l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1 1 0 0 0 .2-1.1 1 1 0 0 0-.9-.6H4a2 2 0 0 1 0-4h.2a1 1 0 0 0 .9-.6 1 1 0 0 0-.2-1.1l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1 1 0 0 0 1.1.2 1 1 0 0 0 .6-.9V4a2 2 0 0 1 4 0v.2a1 1 0 0 0 .6.9 1 1 0 0 0 1.1-.2l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1 1 0 0 0-.2 1.1 1 1 0 0 0 .9.6H20a2 2 0 0 1 0 4h-.2a1 1 0 0 0-.9.6Z" />
    </svg>
  );
}

function ChevronDownIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 9l6 6 6-6" />
    </svg>
  );
}

const NAV_ITEMS: Array<{ page: AppPage; label: string; Icon: () => JSX.Element }> = [
  { page: 'scores', label: 'Scores', Icon: ScoresIcon },
  { page: 'rating', label: 'Rating', Icon: RatingIcon },
  { page: 'playlogs', label: 'Playlogs', Icon: PlaylogsIcon },
  { page: 'picker', label: 'Picker', Icon: PickerIcon },
  { page: 'settings', label: 'Settings', Icon: SettingsIcon },
];

function App() {
  const mobileNavRef = useRef<HTMLElement | null>(null);
  const savedScoreFilters = useMemo(
    () => readStoredJson<StoredScoreFilters>(SCORE_FILTERS_STORAGE_KEY),
    [],
  );
  const savedPlaylogFilters = useMemo(
    () => readStoredJson<StoredPlaylogFilters>(PLAYLOG_FILTERS_STORAGE_KEY),
    [],
  );

  const [activePage, setActivePage] = useState<AppPage>(() => readPageFromHash(window.location.hash));
  const [isMobileNavOpen, setIsMobileNavOpen] = useState(false);

  const [songInfoUrl, setSongInfoUrl] = useState<string>(() =>
    readStoredValue(SONG_INFO_STORAGE_KEY, DEFAULT_SONG_INFO_URL),
  );
  const [recordCollectorUrl, setRecordCollectorUrl] = useState<string>(() =>
    readStoredValue(RECORD_STORAGE_KEY, DEFAULT_RECORD_COLLECTOR_URL),
  );
  const [songInfoUrlDraft, setSongInfoUrlDraft] = useState(songInfoUrl);
  const [recordCollectorUrlDraft, setRecordCollectorUrlDraft] =
    useState(recordCollectorUrl);

  const [isLoading, setIsLoading] = useState(false);
  const [loadingError, setLoadingError] = useState<string | null>(null);

  const [scoreRecords, setScoreRecords] = useState<ScoreApiResponse[]>([]);
  const [playlogRecords, setPlaylogRecords] = useState<PlayRecordApiResponse[]>([]);
  const [songMetadata, setSongMetadata] = useState<Map<string, SongInfoResponse>>(
    () => new Map(),
  );
  const [versionsResponse, setVersionsResponse] = useState<string[]>([]);
  const [pickerVersionOptions, setPickerVersionOptions] = useState<SongVersionResponse[]>([]);

  const [query, setQuery] = useState('');
  const [chartFilter, setChartFilter] = useState<ChartType[]>(() => {
    const values = coerceArray(savedScoreFilters?.chartFilter, CHART_TYPES);
    return values.length > 0 ? values : [...CHART_TYPES];
  });
  const [difficultyFilter, setDifficultyFilter] = useState<DifficultyCategory[]>(() => {
    const values = coerceArray(savedScoreFilters?.difficultyFilter, DIFFICULTIES);
    return values.length > 0 ? values : [...DIFFICULTIES];
  });
  const [versionSelection, setVersionSelection] = useState<string>(() => {
    if (typeof savedScoreFilters?.versionSelection === 'string') {
      return savedScoreFilters.versionSelection;
    }
    const legacy = coerceStringArray(savedScoreFilters?.versionFilter);
    if (legacy.length === 1) {
      return legacy[0];
    }
    return 'ALL';
  });
  const [rankFilter, setRankFilter] = useState<ScoreRank[]>(
    () => {
      const values = coerceArray(savedScoreFilters?.rankFilter, SCORE_RANK_ORDER);
      if (!values.some((value) => AAA_OR_BELOW_RANKS.includes(value))) {
        return values;
      }
      return sortByOrder(Array.from(new Set([...values, ...AAA_OR_BELOW_RANKS])), SCORE_RANK_ORDER_MAP);
    },
  );
  const [fcFilter, setFcFilter] = useState<FcStatus[]>(
    () => coerceArray(savedScoreFilters?.fcFilter, FC_ORDER),
  );
  const [syncFilter, setSyncFilter] = useState<SyncStatus[]>(
    () => coerceArray(savedScoreFilters?.syncFilter, SYNC_ORDER),
  );

  const [achievementMin, setAchievementMin] = useState(() =>
    coerceNumber(savedScoreFilters?.achievementMin, 0),
  );
  const [achievementMax, setAchievementMax] = useState(() =>
    coerceNumber(savedScoreFilters?.achievementMax, 101),
  );
  const [internalMin, setInternalMin] = useState(() =>
    coerceNumber(savedScoreFilters?.internalMin, 1),
  );
  const [internalMax, setInternalMax] = useState(() =>
    coerceNumber(savedScoreFilters?.internalMax, 15.5),
  );
  const [daysMin, setDaysMin] = useState(() => coerceNumber(savedScoreFilters?.daysMin, 0));
  const [daysMax, setDaysMax] = useState(() => coerceNumber(savedScoreFilters?.daysMax, 2000));

  const [scoreSortKey, setScoreSortKey] = useState<ScoreSortKey>('lastPlayed');
  const [scoreSortDesc, setScoreSortDesc] = useState(true);

  const [selectedDetailSongKey, setSelectedDetailSongKey] = useState<string | null>(null);
  const [selectedHistoryKey, setSelectedHistoryKey] = useState<string | null>(null);
  const [showJackets, setShowJackets] = useState<boolean>(readShowJacketsPreference);

  const [playlogQuery, setPlaylogQuery] = useState('');
  const [playlogChartFilter, setPlaylogChartFilter] = useState<ChartType[]>(() => {
    const values = coerceArray(savedPlaylogFilters?.chartFilter, CHART_TYPES);
    return values.length > 0 ? values : [...CHART_TYPES];
  });
  const [playlogDifficultyFilter, setPlaylogDifficultyFilter] = useState<DifficultyCategory[]>(() => {
    const values = coerceArray(savedPlaylogFilters?.difficultyFilter, DIFFICULTIES);
    return values.length > 0 ? values : [...DIFFICULTIES];
  });
  const [playlogAchievementMin, setPlaylogAchievementMin] = useState(() =>
    coerceNumber(savedPlaylogFilters?.achievementMin, 0),
  );
  const [playlogAchievementMax, setPlaylogAchievementMax] = useState(() =>
    coerceNumber(savedPlaylogFilters?.achievementMax, 101),
  );
  const [playlogSortKey, setPlaylogSortKey] = useState<PlaylogSortKey>('playedAt');
  const [playlogSortDesc, setPlaylogSortDesc] = useState(true);
  const [isPlaylogDateFilterDisabled, setIsPlaylogDateFilterDisabled] = useState(false);
  const [selectedPlaylogDayKey, setSelectedPlaylogDayKey] = useState<string | null>(null);

  const loadAbortRef = useRef<AbortController | null>(null);
  const loadedExplorerKeyRef = useRef<string | null>(null);

  const scoreData = useMemo(
    () => buildScoreRows(scoreRecords, songMetadata),
    [scoreRecords, songMetadata],
  );
  const selectedDetailRows = useMemo(
    () => buildSongDetailRows(scoreData, selectedDetailSongKey),
    [scoreData, selectedDetailSongKey],
  );
  const selectedDetailSong = selectedDetailRows[0] ?? null;
  const playlogData = useMemo(
    () => buildPlaylogRows(playlogRecords, songMetadata),
    [playlogRecords, songMetadata],
  );
  const selectedHistoryRow = useMemo(
    () => scoreData.find((row) => row.key === selectedHistoryKey) ?? null,
    [scoreData, selectedHistoryKey],
  );
  const selectedHistoryPoints = useMemo(
    () => buildScoreHistoryPoints(playlogData, selectedHistoryRow),
    [playlogData, selectedHistoryRow],
  );

  const versionOptions = useMemo(() => {
    if (versionsResponse.length > 0) {
      return sortByOrder(versionsResponse, VERSION_ORDER_MAP);
    }

    return sortByOrder(
      Array.from(
        new Set(scoreData.map((row) => row.version).filter((value): value is string => Boolean(value))),
      ),
      VERSION_ORDER_MAP,
    );
  }, [scoreData, versionsResponse]);

  const loadData = useCallback(async () => {
    if (!songInfoUrl.trim() || !recordCollectorUrl.trim()) {
      loadedExplorerKeyRef.current = null;
      setIsLoading(false);
      setScoreRecords([]);
      setPlaylogRecords([]);
      setVersionsResponse([]);
      setPickerVersionOptions([]);
      setSongMetadata(new Map<string, SongInfoResponse>());
      setLoadingError('Scores와 Playlogs 페이지는 Song Info와 Record Collector URL이 모두 필요합니다.');
      return;
    }

    const requestKey = `${songInfoUrl.trim()}::${recordCollectorUrl.trim()}`;
    if (loadedExplorerKeyRef.current === requestKey) {
      setIsLoading(false);
      setLoadingError(null);
      return;
    }

    loadAbortRef.current?.abort();
    const controller = new AbortController();
    loadAbortRef.current = controller;

    setIsLoading(true);
    setLoadingError(null);
    setSongMetadata(new Map<string, SongInfoResponse>());

    try {
      const payload = await fetchExplorerPayload(
        songInfoUrl,
        recordCollectorUrl,
        controller.signal,
      );

      if (controller.signal.aborted) {
        return;
      }

      setScoreRecords(payload.ratedScores);
      setPlaylogRecords(payload.playlogs);
      setSongMetadata(payload.songMetadata);
      setVersionsResponse(
        payload.versions?.versions
          .map((version) => version.version_name)
          .filter((name) => name.length > 0) ?? [],
      );
      setPickerVersionOptions(payload.versions?.versions ?? []);
      loadedExplorerKeyRef.current = requestKey;
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }
      loadedExplorerKeyRef.current = null;
      const message = error instanceof Error ? error.message : String(error);
      setLoadingError(message);
      setScoreRecords([]);
      setPlaylogRecords([]);
      setSongMetadata(new Map<string, SongInfoResponse>());
      setVersionsResponse([]);
      setPickerVersionOptions([]);
    } finally {
      if (!controller.signal.aborted) {
        setIsLoading(false);
      }
    }
  }, [recordCollectorUrl, songInfoUrl]);

  useEffect(() => {
    void loadData();

    return () => {
      loadAbortRef.current?.abort();
    };
  }, [loadData]);

  useEffect(() => {
    const onHashChange = () => {
      setActivePage(readPageFromHash(window.location.hash));
    };

    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, []);

  useEffect(() => {
    if (
      versionSelection === 'ALL' ||
      versionSelection === 'NEW' ||
      versionSelection === 'OLD' ||
      versionOptions.includes(versionSelection)
    ) {
      return;
    }
    setVersionSelection('ALL');
  }, [versionOptions, versionSelection]);

  useEffect(() => {
    const payload: StoredScoreFilters = {
      chartFilter,
      difficultyFilter,
      versionSelection,
      rankFilter,
      fcFilter,
      syncFilter,
      achievementMin,
      achievementMax,
      internalMin,
      internalMax,
      daysMin,
      daysMax,
    };
    localStorage.setItem(SCORE_FILTERS_STORAGE_KEY, JSON.stringify(payload));
  }, [
    achievementMax,
    achievementMin,
    chartFilter,
    daysMax,
    daysMin,
    difficultyFilter,
    fcFilter,
    internalMax,
    internalMin,
    rankFilter,
    syncFilter,
    versionSelection,
  ]);

  useEffect(() => {
    const payload: StoredPlaylogFilters = {
      chartFilter: playlogChartFilter,
      difficultyFilter: playlogDifficultyFilter,
      achievementMin: playlogAchievementMin,
      achievementMax: playlogAchievementMax,
    };
    localStorage.setItem(PLAYLOG_FILTERS_STORAGE_KEY, JSON.stringify(payload));
  }, [
    playlogAchievementMax,
    playlogAchievementMin,
    playlogChartFilter,
    playlogDifficultyFilter,
  ]);

  useEffect(() => {
    localStorage.setItem(SONG_INFO_STORAGE_KEY, songInfoUrl);
  }, [songInfoUrl]);

  useEffect(() => {
    localStorage.setItem(RECORD_STORAGE_KEY, recordCollectorUrl);
  }, [recordCollectorUrl]);

  useEffect(() => {
    localStorage.setItem(TABLE_LAYOUT_STORAGE_KEY, showJackets ? 'jacket' : 'compact');
  }, [showJackets]);

  useEffect(() => {
    if (activePage !== 'settings') {
      return;
    }
    setSongInfoUrlDraft(songInfoUrl);
    setRecordCollectorUrlDraft(recordCollectorUrl);
  }, [activePage, recordCollectorUrl, songInfoUrl]);

  useEffect(() => {
    setIsMobileNavOpen(false);
  }, [activePage]);

  useEffect(() => {
    if (!isMobileNavOpen) {
      return undefined;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (mobileNavRef.current?.contains(event.target as Node)) {
        return;
      }
      setIsMobileNavOpen(false);
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsMobileNavOpen(false);
      }
    };

    document.addEventListener('pointerdown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isMobileNavOpen]);

  const handleOpenSongDetail = useCallback((row: ScoreRow) => {
    setSelectedDetailSongKey(songIdentityKey(row.title, row.genre, row.artist));
  }, []);

  const closeSongDetail = useCallback(() => {
    setSelectedDetailSongKey(null);
  }, []);

  const handleOpenHistory = useCallback((row: ScoreRow) => {
    setSelectedHistoryKey(row.key);
  }, []);

  const closeHistory = useCallback(() => {
    setSelectedHistoryKey(null);
  }, []);

  const handleScoreSortBy = useCallback(
    (key: ScoreSortKey) => {
      if (scoreSortKey === key) {
        setScoreSortDesc((current) => !current);
        return;
      }
      setScoreSortKey(key);
      setScoreSortDesc(key !== 'title');
    },
    [scoreSortKey],
  );

  const handlePlaylogSortBy = useCallback(
    (key: PlaylogSortKey) => {
      if (playlogSortKey === key) {
        setPlaylogSortDesc((current) => !current);
        return;
      }
      setPlaylogSortKey(key);
      setPlaylogSortDesc(key !== 'title');
    },
    [playlogSortKey],
  );

  const scoreRankOptions = useMemo(() => computeScoreRankOptions(scoreData), [scoreData]);
  const fcOptions = useMemo(() => computeFcOptions(scoreData), [scoreData]);
  const syncOptions = useMemo(() => computeSyncOptions(scoreData), [scoreData]);
  const rankFilterOptions = useMemo(() => {
    const next: string[] = [];
    let groupedAdded = false;
    for (const rank of scoreRankOptions) {
      if (AAA_OR_BELOW_RANKS.includes(rank)) {
        if (!groupedAdded) {
          next.push(RANK_FILTER_AAA_OR_BELOW_LABEL);
          groupedAdded = true;
        }
        continue;
      }
      next.push(rank);
    }
    return next;
  }, [scoreRankOptions]);

  const selectedRankFilterOptions = useMemo(() => {
    const next: string[] = [];
    const hasGroupedOption = rankFilterOptions.includes(RANK_FILTER_AAA_OR_BELOW_LABEL);
    if (hasGroupedOption && AAA_OR_BELOW_RANKS.every((rank) => rankFilter.includes(rank))) {
      next.push(RANK_FILTER_AAA_OR_BELOW_LABEL);
    }
    for (const rank of scoreRankOptions) {
      if (!AAA_OR_BELOW_RANKS.includes(rank) && rankFilter.includes(rank)) {
        next.push(rank);
      }
    }
    return next;
  }, [rankFilter, rankFilterOptions, scoreRankOptions]);

  const handleRankFilterToggle = useCallback((value: string) => {
    if (value === RANK_FILTER_AAA_OR_BELOW_LABEL) {
      setRankFilter((current) => {
        if (AAA_OR_BELOW_RANKS.every((rank) => current.includes(rank))) {
          return current.filter((rank) => !AAA_OR_BELOW_RANKS.includes(rank));
        }
        return sortByOrder(
          Array.from(new Set([...current, ...AAA_OR_BELOW_RANKS])),
          SCORE_RANK_ORDER_MAP,
        );
      });
      return;
    }

    if (!SCORE_RANK_ORDER.includes(value as ScoreRank)) {
      return;
    }
    setRankFilter((current) => toggleArrayValue(current, value as ScoreRank));
  }, []);

  const filteredScoreRows = useMemo(
    () =>
      buildFilteredScoreRows({
        scoreData,
        query,
        chartFilter,
        difficultyFilter,
        versionSelection,
        versionOptions,
        rankFilter,
        fcFilter,
        syncFilter,
        achievementMin,
        achievementMax,
        internalMin,
        internalMax,
        daysMin,
        daysMax,
        scoreSortKey,
        scoreSortDesc,
      }),
    [
      achievementMax,
      achievementMin,
      chartFilter,
      daysMax,
      daysMin,
      difficultyFilter,
      fcFilter,
      internalMax,
      internalMin,
      query,
      rankFilter,
      scoreData,
      scoreSortDesc,
      scoreSortKey,
      syncFilter,
      versionOptions,
      versionSelection,
    ],
  );

  const availablePlaylogDayKeys = useMemo(() => {
    const dayKeys = new Set(playlogData.map((row) => toMaimaiDayKey(row.playedAtUnix)));
    return Array.from(dayKeys).sort((left, right) => right.localeCompare(left));
  }, [playlogData]);

  useEffect(() => {
    if (availablePlaylogDayKeys.length === 0) {
      setSelectedPlaylogDayKey(null);
      return;
    }

    const latestDayKey = availablePlaylogDayKeys[0];
    setSelectedPlaylogDayKey((current) => {
      if (current && availablePlaylogDayKeys.includes(current)) {
        return current;
      }
      return latestDayKey;
    });
  }, [availablePlaylogDayKeys]);

  const selectedPlaylogDayStartUnix = useMemo(() => {
    if (isPlaylogDateFilterDisabled || !selectedPlaylogDayKey) {
      return null;
    }
    return toMaimaiDayStartUnix(selectedPlaylogDayKey);
  }, [isPlaylogDateFilterDisabled, selectedPlaylogDayKey]);

  const selectedPlaylogDayEndUnix = useMemo(() => {
    if (selectedPlaylogDayStartUnix === null) {
      return null;
    }
    return selectedPlaylogDayStartUnix + 24 * 60 * 60;
  }, [selectedPlaylogDayStartUnix]);

  const selectedPlaylogDayRows = useMemo(() => {
    if (selectedPlaylogDayStartUnix === null || selectedPlaylogDayEndUnix === null) {
      return playlogData;
    }
    return playlogData.filter(
      (row) => row.playedAtUnix >= selectedPlaylogDayStartUnix && row.playedAtUnix < selectedPlaylogDayEndUnix,
    );
  }, [playlogData, selectedPlaylogDayEndUnix, selectedPlaylogDayStartUnix]);

  const selectedPlaylogDayCreditCount = useMemo(() => {
    return countCredits(selectedPlaylogDayRows);
  }, [selectedPlaylogDayRows]);

  const playlogDayOptions = useMemo(() => {
    return availablePlaylogDayKeys.map((dayKey) => {
      const dayStartUnix = toMaimaiDayStartUnix(dayKey);
      if (dayStartUnix === null) {
        return { key: dayKey, creditCount: null };
      }

      const dayEndUnix = dayStartUnix + 24 * 60 * 60;
      const dayRows = playlogData.filter((row) => row.playedAtUnix >= dayStartUnix && row.playedAtUnix < dayEndUnix);
      return { key: dayKey, creditCount: countCredits(dayRows) };
    });
  }, [availablePlaylogDayKeys, playlogData]);

  const selectedPlaylogDaySongCount = selectedPlaylogDayRows.length;

  const filteredPlaylogRows = useMemo(
    () =>
      buildFilteredPlaylogRows({
        playlogData,
        playlogQuery,
        playlogChartFilter,
        playlogDifficultyFilter,
        playlogAchievementMin,
        playlogAchievementMax,
        playlogSortKey,
        playlogSortDesc,
        playlogDayStartUnix: selectedPlaylogDayStartUnix,
        playlogDayEndUnix: selectedPlaylogDayEndUnix,
      }),
    [
      playlogAchievementMax,
      playlogAchievementMin,
      playlogChartFilter,
      playlogData,
      playlogDifficultyFilter,
      playlogQuery,
      playlogSortDesc,
      playlogSortKey,
      selectedPlaylogDayEndUnix,
      selectedPlaylogDayStartUnix,
    ],
  );

  const { ratingTotal, newRatingTotal, oldRatingTotal, newRatingRows, oldRatingRows } = useMemo(() => {
    const latestVersions = versionOptions.slice(-2);
    const latestSet = new Set(latestVersions);

    const classifiedRows = scoreData.filter(
      (row): row is ScoreRow & { ratingPoints: number; version: string } =>
        row.ratingPoints !== null && row.version !== null,
    );
    const newRows = classifiedRows
      .filter((row) => latestSet.has(row.version))
      .sort((left, right) => right.ratingPoints - left.ratingPoints)
      .slice(0, 15);
    const oldRows = classifiedRows
      .filter((row) => !latestSet.has(row.version))
      .sort((left, right) => right.ratingPoints - left.ratingPoints)
      .slice(0, 35);

    const newTotal = newRows.reduce((sum, row) => sum + (row.ratingPoints ?? 0), 0);
    const oldTotal = oldRows.reduce((sum, row) => sum + (row.ratingPoints ?? 0), 0);

    return {
      ratingTotal: newTotal + oldTotal,
      newRatingTotal: newTotal,
      oldRatingTotal: oldTotal,
      newRatingRows: newRows,
      oldRatingRows: oldRows,
    };
  }, [scoreData, versionOptions]);

  const handleApplyUrls = () => {
    const nextSongInfoUrl = songInfoUrlDraft.trim();
    const nextRecordUrl = recordCollectorUrlDraft.trim();

    if (!nextSongInfoUrl) {
      return;
    }

    setSongInfoUrl(nextSongInfoUrl);
    setRecordCollectorUrl(nextRecordUrl);
  };

  const handleNavigatePage = useCallback((page: AppPage) => {
    if (page === 'settings') {
      setSongInfoUrlDraft(songInfoUrl);
      setRecordCollectorUrlDraft(recordCollectorUrl);
    }
    const nextHash = page === 'playlogs'
      ? '#playlogs'
      : page === 'rating'
        ? '#rating'
      : page === 'picker'
        ? '#picker'
        : page === 'settings'
          ? '#settings'
          : '#scores';
    if (window.location.hash !== nextHash) {
      window.location.hash = nextHash;
      return;
    }
    setActivePage(page);
  }, [recordCollectorUrl, songInfoUrl]);

  const scoreCountLabel = `${filteredScoreRows.length.toLocaleString()}/${scoreData.length.toLocaleString()}`;
  const playlogCountLabel = `${filteredPlaylogRows.length.toLocaleString()}/${playlogData.length.toLocaleString()}`;
  const activeNavItem = NAV_ITEMS.find((item) => item.page === activePage) ?? NAV_ITEMS[0];
  const ActiveNavIcon = activeNavItem.Icon;
  const mobileNavItems = NAV_ITEMS;
  const desktopSidebarTopContent = (
    <section className="panel app-sidebar-inline">
      <div className="brand-copy">
        <h1>maistats</h1>
      </div>

      <nav className="app-nav app-nav-inline" aria-label="Primary">
        <div className="app-nav-list">
          {NAV_ITEMS.map(({ page, label, Icon }) => (
            <button
              key={page}
              type="button"
              className={activePage === page ? 'active' : ''}
              onClick={() => handleNavigatePage(page)}
            >
              <Icon />
              <span>{label}</span>
            </button>
          ))}
        </div>
      </nav>
    </section>
  );

  return (
    <div className="app-shell">
      <aside className="app-sidebar panel">
        <div className="brand-copy">
          <h1>maistats</h1>
        </div>

        <nav ref={mobileNavRef} className="app-nav" aria-label="Primary">
          <button
            type="button"
            className="app-nav-trigger"
            aria-expanded={isMobileNavOpen}
            aria-label="페이지 목록 열기"
            onClick={() => setIsMobileNavOpen((current) => !current)}
          >
            <span className="app-nav-trigger-main">
              <ActiveNavIcon />
              <span>{activeNavItem.label}</span>
            </span>
            <span className={`app-nav-trigger-chevron ${isMobileNavOpen ? 'is-open' : ''}`}>
              <ChevronDownIcon />
            </span>
          </button>
          <div className="app-nav-list app-nav-list-desktop">
            {NAV_ITEMS.map(({ page, label, Icon }) => (
              <button
                key={page}
                type="button"
                className={activePage === page ? 'active' : ''}
                onClick={() => handleNavigatePage(page)}
              >
                <Icon />
                <span>{label}</span>
              </button>
            ))}
          </div>
          <div className={`app-nav-list app-nav-list-mobile ${isMobileNavOpen ? 'is-open' : ''}`}>
            {mobileNavItems.map(({ page, label, Icon }) => (
              <button
                key={page}
                type="button"
                className={activePage === page ? 'active' : ''}
                onClick={() => handleNavigatePage(page)}
              >
                <Icon />
                <span>{label}</span>
              </button>
            ))}
          </div>
        </nav>
      </aside>

      <main className="app-main">
        {activePage === 'scores' ? (
          <>
            {loadingError ? <section className="error-banner">에러: {loadingError}</section> : null}

            <ScoreExplorerSection
              sidebarTopContent={desktopSidebarTopContent}
              scoreCountLabel={scoreCountLabel}
              isLoading={isLoading}
              showJackets={showJackets}
              setShowJackets={setShowJackets}
              query={query}
              setQuery={setQuery}
              chartTypes={CHART_TYPES}
              chartFilter={chartFilter}
              setChartFilter={setChartFilter}
              difficulties={DIFFICULTIES}
              difficultyFilter={difficultyFilter}
              setDifficultyFilter={setDifficultyFilter}
              versionOptions={versionOptions}
              versionSelection={versionSelection}
              setVersionSelection={setVersionSelection}
              scoreRankOptions={rankFilterOptions}
              rankFilter={selectedRankFilterOptions}
              onToggleRankFilter={handleRankFilterToggle}
              fcOptions={fcOptions}
              fcFilter={fcFilter}
              setFcFilter={setFcFilter}
              syncOptions={syncOptions}
              syncFilter={syncFilter}
              setSyncFilter={setSyncFilter}
              achievementMin={achievementMin}
              setAchievementMin={setAchievementMin}
              achievementMax={achievementMax}
              setAchievementMax={setAchievementMax}
              internalMin={internalMin}
              setInternalMin={setInternalMin}
              internalMax={internalMax}
              setInternalMax={setInternalMax}
              daysMin={daysMin}
              setDaysMin={setDaysMin}
              daysMax={daysMax}
              setDaysMax={setDaysMax}
              filteredScoreRows={filteredScoreRows}
              songInfoUrl={songInfoUrl}
              onOpenSongDetail={handleOpenSongDetail}
              onOpenHistory={handleOpenHistory}
              scoreSortKey={scoreSortKey}
              scoreSortDesc={scoreSortDesc}
              onSortBy={handleScoreSortBy}
            />
          </>
        ) : activePage === 'playlogs' ? (
          <>
            {loadingError ? <section className="error-banner">에러: {loadingError}</section> : null}

            <PlaylogExplorerSection
              sidebarTopContent={desktopSidebarTopContent}
              playlogCountLabel={playlogCountLabel}
              showJackets={showJackets}
              setShowJackets={setShowJackets}
              playlogQuery={playlogQuery}
              setPlaylogQuery={setPlaylogQuery}
              chartTypes={CHART_TYPES}
              playlogChartFilter={playlogChartFilter}
              setPlaylogChartFilter={setPlaylogChartFilter}
              difficulties={DIFFICULTIES}
              playlogDifficultyFilter={playlogDifficultyFilter}
              setPlaylogDifficultyFilter={setPlaylogDifficultyFilter}
              playlogAchievementMin={playlogAchievementMin}
              setPlaylogAchievementMin={setPlaylogAchievementMin}
              playlogAchievementMax={playlogAchievementMax}
              setPlaylogAchievementMax={setPlaylogAchievementMax}
              isPlaylogDateFilterDisabled={isPlaylogDateFilterDisabled}
              setIsPlaylogDateFilterDisabled={setIsPlaylogDateFilterDisabled}
              selectedPlaylogDayKey={selectedPlaylogDayKey}
              setSelectedPlaylogDayKey={setSelectedPlaylogDayKey}
              playlogDayOptions={playlogDayOptions}
              selectedPlaylogDayCreditCount={selectedPlaylogDayCreditCount}
              selectedPlaylogDaySongCount={selectedPlaylogDaySongCount}
              filteredPlaylogRows={filteredPlaylogRows}
              songInfoUrl={songInfoUrl}
              playlogSortKey={playlogSortKey}
              playlogSortDesc={playlogSortDesc}
              onSortBy={handlePlaylogSortBy}
            />
          </>
        ) : activePage === 'rating' ? (
          <>
            {loadingError ? <section className="error-banner">에러: {loadingError}</section> : null}
            <RatingPage
              sidebarTopContent={desktopSidebarTopContent}
              songInfoUrl={songInfoUrl}
              ratingTotal={ratingTotal}
              newRatingTotal={newRatingTotal}
              oldRatingTotal={oldRatingTotal}
              newRows={newRatingRows}
              oldRows={oldRatingRows}
            />
          </>
        ) : activePage === 'picker' ? (
          <RandomPickerPage
            sidebarTopContent={desktopSidebarTopContent}
            songInfoUrl={songInfoUrl}
            scoreRecords={scoreRecords}
            songMetadata={songMetadata}
            versionOptions={pickerVersionOptions}
          />
        ) : (
          <SettingsPage
            sidebarTopContent={desktopSidebarTopContent}
            songInfoUrl={songInfoUrl}
            recordCollectorUrl={recordCollectorUrl}
            songInfoUrlDraft={songInfoUrlDraft}
            setSongInfoUrlDraft={setSongInfoUrlDraft}
            recordCollectorUrlDraft={recordCollectorUrlDraft}
            setRecordCollectorUrlDraft={setRecordCollectorUrlDraft}
            onApply={handleApplyUrls}
          />
        )}
      </main>

      <SongDetailModal
        selectedDetailTitle={selectedDetailSong?.title ?? null}
        selectedDetailGenre={selectedDetailSong?.genre ?? null}
        selectedDetailArtist={selectedDetailSong?.artist ?? null}
        selectedDetailRows={selectedDetailRows}
        songInfoUrl={songInfoUrl}
        onClose={closeSongDetail}
      />
      <ScoreHistoryModal
        selectedHistoryRow={selectedHistoryRow}
        historyPoints={selectedHistoryPoints}
        songInfoUrl={songInfoUrl}
        onClose={closeHistory}
      />
    </div>
  );
}

export default App;
