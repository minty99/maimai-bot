import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import {
  fetchExplorerPayload,
  fetchRecentPlaylogs,
  type PlayerProfile,
  refreshSongScores,
} from './api';
import { HomePage } from './components/HomePage';
import {
  CHART_TYPES,
  DEFAULT_SONG_DATABASE_URL,
  DIFFICULTIES,
  PLAYLOG_FILTERS_STORAGE_KEY,
  PlaylogSortKey,
  RECORD_STORAGE_KEY,
  SCORE_FILTERS_STORAGE_KEY,
  SONG_DATABASE_STORAGE_KEY,
  SONG_INFO_STORAGE_KEY,
  TABLE_LAYOUT_STORAGE_KEY,
  THEME_STORAGE_KEY,
  ScoreSortKey,
  VERSION_ORDER_MAP,
} from './app/constants';
import { filterAvailableVersions, sortByOrder } from './app/utils';
import {
  ALL_FILTER_PRESET_ID,
  DEFAULT_SCORE_FILTERS,
  DirectionalRangeSelectionState,
  FC_FILTER_OPTIONS,
  getPresetSelectionRange,
  INTERNAL_LEVEL_CONTIGUOUS_PRESET_ORDER,
  InternalLevelPresetId,
  INTERNAL_LEVEL_PRESETS,
  resolvePresetSelectionFromRange,
  SCORE_ACHIEVEMENT_PRESET_ORDER,
  SCORE_ACHIEVEMENT_PRESETS,
  ScoreAchievementPresetId,
  SyncFilterOptionId,
  FcFilterOptionId,
  SYNC_FILTER_OPTIONS,
  updateDirectionalRangeSelection,
} from './app/scoreFilterPresets';
import {
  coerceArray,
  coerceNumber,
  coerceStringArray,
  readStoredJson,
  StoredPlaylogFilters,
  StoredScoreFilters,
} from './app/storage';
import {
  buildPlaylogRows,
  buildScoreHistoryPoints,
  buildSongDetailRows,
  buildScoreRows,
  toIntegerRating,
} from './derive';
import { useI18n, type TranslationKey, type TranslationVariables } from './app/i18n';
import {
  buildFilteredPlaylogRows,
  buildFilteredScoreRows,
} from './app/filtering';
import { PlaylogExplorerSection } from './components/PlaylogExplorerSection';
import { RandomPickerPage } from './components/RandomPickerPage';
import { RatingPage } from './components/RatingPage';
import { ScoreExplorerSection } from './components/ScoreExplorerSection';
import { SettingsPage } from './components/SettingsPage';
import { SetupGuidePage } from './components/SetupGuidePage';
import { SongDetailModal } from './components/SongDetailModal';
import { ScoreHistoryModal } from './components/ScoreHistoryModal';
import type { SongDetailTarget } from './components/TableActionCells';
import { songIdentityKey } from './songIdentity';
import type {
  ChartType,
  DifficultyCategory,
  PlayRecordApiResponse,
  PlaylogRow,
  ScoreApiResponse,
  ScoreRow,
  SongInfoResponse,
  SongVersionResponse,
} from './types';
import logoUrl from './assets/logo.png';

type AppPage = 'home' | 'setup' | 'scores' | 'rating' | 'playlogs' | 'picker' | 'settings';
type RatedScoreRow = ScoreRow & { rating: number; version: string };
type ThemePreference = 'system' | 'light' | 'dark';
type LoadingErrorState =
  | { kind: 'translated'; key: TranslationKey; variables?: TranslationVariables }
  | { kind: 'message'; message: string };

function readPageFromHash(hash: string): AppPage {
  if (hash === '#home') {
    return 'home';
  }
  if (hash === '#rating') {
    return 'rating';
  }
  if (hash === '#setup') {
    return 'setup';
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

function compareRatingPageRows(
  left: RatedScoreRow,
  right: RatedScoreRow,
  locale: string,
): number {
  const ratingDiff = right.rating - left.rating;
  if (ratingDiff !== 0) {
    return ratingDiff;
  }

  return left.title.localeCompare(right.title, locale);
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

function HomeIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
      <path d="M9 22V12h6v10" />
    </svg>
  );
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

function SetupIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 4h12" />
      <path d="M6 9h12" />
      <path d="M6 14h8" />
      <path d="M6 19h8" />
      <path d="M18 14v5" />
      <path d="M15.5 16.5 18 14l2.5 2.5" />
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

function App() {
  const {
    formatLanguageName,
    formatNumber,
    language,
    languagePreference,
    locale,
    setLanguagePreference,
    t,
  } = useI18n();
  const mobileNavRef = useRef<HTMLElement | null>(null);
  const savedScoreFilters = useMemo(
    () => readStoredJson<StoredScoreFilters>(SCORE_FILTERS_STORAGE_KEY),
    [],
  );
  const savedPlaylogFilters = useMemo(
    () => readStoredJson<StoredPlaylogFilters>(PLAYLOG_FILTERS_STORAGE_KEY),
    [],
  );

  const [activePage, setActivePage] = useState<AppPage>(() => {
    const stored = localStorage.getItem(RECORD_STORAGE_KEY)?.trim();
    const envUrl = (import.meta.env.RECORD_COLLECTOR_SERVER_URL as string | undefined)?.trim();
    const hasUrl = Boolean(stored ?? envUrl);
    const requestedPage = readPageFromHash(window.location.hash);
    if (!hasUrl && requestedPage !== 'home' && requestedPage !== 'setup' && requestedPage !== 'settings') {
      return 'home';
    }
    return requestedPage;
  });
  const [isMobileNavOpen, setIsMobileNavOpen] = useState(false);

  const [themePreference, setThemePreferenceState] = useState<ThemePreference>(() => {
    const stored = localStorage.getItem(THEME_STORAGE_KEY)?.trim();
    if (stored === 'light' || stored === 'dark') return stored;
    return 'system';
  });

  useEffect(() => {
    localStorage.setItem(THEME_STORAGE_KEY, themePreference);
    if (themePreference === 'system') {
      document.documentElement.removeAttribute('data-theme');
    } else {
      document.documentElement.setAttribute('data-theme', themePreference);
    }
  }, [themePreference]);

  const [songInfoUrl, setSongInfoUrl] = useState<string>(DEFAULT_SONG_DATABASE_URL);
  const [recordCollectorUrl, setRecordCollectorUrl] = useState<string>(() => {
    const stored = localStorage.getItem(RECORD_STORAGE_KEY)?.trim();
    if (stored) return stored;
    return (import.meta.env.RECORD_COLLECTOR_SERVER_URL as string | undefined)?.trim() ?? '';
  });
  const [songInfoUrlDraft, setSongInfoUrlDraft] = useState(songInfoUrl);
  const [recordCollectorUrlDraft, setRecordCollectorUrlDraft] =
    useState(recordCollectorUrl);

  const [isLoading, setIsLoading] = useState(false);
  const [isPlaylogsLoading, setIsPlaylogsLoading] = useState(false);
  const [loadingError, setLoadingError] = useState<LoadingErrorState | null>(null);
  const [playlogLoadingError, setPlaylogLoadingError] = useState<LoadingErrorState | null>(null);

  const [scoreRecords, setScoreRecords] = useState<ScoreApiResponse[]>([]);
  const [playlogRecords, setPlaylogRecords] = useState<PlayRecordApiResponse[]>([]);
  const [songMetadata, setSongMetadata] = useState<Map<string, SongInfoResponse>>(
    () => new Map(),
  );

  useEffect(() => {
    localStorage.removeItem(SONG_DATABASE_STORAGE_KEY);
    localStorage.removeItem(SONG_INFO_STORAGE_KEY);
  }, []);
  const [versionsResponse, setVersionsResponse] = useState<string[]>([]);
  const [pickerVersionOptions, setPickerVersionOptions] = useState<SongVersionResponse[]>([]);
  const [playerProfile, setPlayerProfile] = useState<PlayerProfile | null>(null);

  const [query, setQuery] = useState('');
  const [scoreQueryDraft, setScoreQueryDraft] = useState(query);
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
    return DEFAULT_SCORE_FILTERS.versionSelection;
  });
  const [fcFilter, setFcFilter] = useState<FcFilterOptionId[]>(() => {
    const values = coerceStringArray(savedScoreFilters?.fcFilter).filter((value): value is FcFilterOptionId =>
      FC_FILTER_OPTIONS.includes(value as FcFilterOptionId),
    );
    return values.length > 0 ? values : [ALL_FILTER_PRESET_ID];
  });
  const [syncFilter, setSyncFilter] = useState<SyncFilterOptionId[]>(() => {
    const values = coerceStringArray(savedScoreFilters?.syncFilter).filter((value): value is SyncFilterOptionId =>
      SYNC_FILTER_OPTIONS.includes(value as SyncFilterOptionId),
    );
    return values.length > 0 ? values : [ALL_FILTER_PRESET_ID];
  });

  const [achievementMin, setAchievementMin] = useState(() =>
    coerceNumber(savedScoreFilters?.achievementMin, DEFAULT_SCORE_FILTERS.achievementMin),
  );
  const [achievementMax, setAchievementMax] = useState(() =>
    coerceNumber(savedScoreFilters?.achievementMax, DEFAULT_SCORE_FILTERS.achievementMax),
  );
  const [internalMin, setInternalMin] = useState(() =>
    coerceNumber(savedScoreFilters?.internalMin, DEFAULT_SCORE_FILTERS.internalMin),
  );
  const [internalMax, setInternalMax] = useState(() =>
    coerceNumber(savedScoreFilters?.internalMax, DEFAULT_SCORE_FILTERS.internalMax),
  );
  const [daysMin, setDaysMin] = useState(() =>
    coerceNumber(savedScoreFilters?.daysMin, DEFAULT_SCORE_FILTERS.daysMin),
  );
  const [daysMax, setDaysMax] = useState(() =>
    coerceNumber(savedScoreFilters?.daysMax, DEFAULT_SCORE_FILTERS.daysMax),
  );
  const [internalLevelSelectionState, setInternalLevelSelectionState] = useState<
    DirectionalRangeSelectionState<Exclude<InternalLevelPresetId, 'ALL'>> | null
  >(null);
  const [scoreAchievementSelectionState, setScoreAchievementSelectionState] = useState<
    DirectionalRangeSelectionState<Exclude<ScoreAchievementPresetId, 'ALL'>> | null
  >(null);
  const [fcSelectionState, setFcSelectionState] = useState<
    DirectionalRangeSelectionState<Exclude<FcFilterOptionId, 'ALL'>> | null
  >(null);
  const [syncSelectionState, setSyncSelectionState] = useState<
    DirectionalRangeSelectionState<Exclude<SyncFilterOptionId, 'ALL'>> | null
  >(null);

  const [scoreSortKey, setScoreSortKey] = useState<ScoreSortKey>('lastPlayed');
  const [scoreSortDesc, setScoreSortDesc] = useState(true);

  const [selectedDetailSongKey, setSelectedDetailSongKey] = useState<string | null>(null);
  const [selectedHistoryKey, setSelectedHistoryKey] = useState<string | null>(null);
  const [showJackets, setShowJackets] = useState<boolean>(readShowJacketsPreference);

  const [playlogQuery, setPlaylogQuery] = useState('');
  const [playlogQueryDraft, setPlaylogQueryDraft] = useState(playlogQuery);
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
  const [playlogBestOnly, setPlaylogBestOnly] = useState(
    savedPlaylogFilters?.bestOnly === true,
  );
  const [playlogNewRecordOnly, setPlaylogNewRecordOnly] = useState(
    savedPlaylogFilters?.newRecordOnly === true,
  );
  const [playlogSortKey, setPlaylogSortKey] = useState<PlaylogSortKey>('playedAt');
  const [playlogSortDesc, setPlaylogSortDesc] = useState(true);
  const [isPlaylogDateFilterDisabled, setIsPlaylogDateFilterDisabled] = useState(false);
  const [selectedPlaylogDayKey, setSelectedPlaylogDayKey] = useState<string | null>(null);

  const loadAbortRef = useRef<AbortController | null>(null);
  const playlogLoadAbortRef = useRef<AbortController | null>(null);
  const loadedExplorerKeyRef = useRef<string | null>(null);
  const loadedPlaylogsKeyRef = useRef<string | null>(null);
  const playlogRecordCountRef = useRef(0);

  useEffect(() => {
    const selector = 'link[rel="icon"]';
    const existing = document.head.querySelector<HTMLLinkElement>(selector);
    const link = existing ?? document.createElement('link');
    link.rel = 'icon';
    link.type = 'image/png';
    link.href = logoUrl;

    if (!existing) {
      document.head.appendChild(link);
    }
  }, []);

  useEffect(() => {
    playlogRecordCountRef.current = playlogRecords.length;
  }, [playlogRecords.length]);

  const scoreData = useMemo(
    () => buildScoreRows(scoreRecords, songMetadata, locale),
    [locale, scoreRecords, songMetadata],
  );
  const selectedDetailRows = useMemo(
    () => buildSongDetailRows(scoreData, selectedDetailSongKey),
    [scoreData, selectedDetailSongKey],
  );
  const selectedDetailSong = selectedDetailRows[0] ?? null;
  const playlogData = useMemo(
    () => buildPlaylogRows(playlogRecords, songMetadata, locale),
    [locale, playlogRecords, songMetadata],
  );
  const selectedHistoryRow = useMemo(
    () => scoreData.find((row) => row.key === selectedHistoryKey) ?? null,
    [scoreData, selectedHistoryKey],
  );
  const selectedHistoryPoints = useMemo(
    () => buildScoreHistoryPoints(playlogData, selectedHistoryRow),
    [playlogData, selectedHistoryRow],
  );
  const scoreRowsByTitle = useMemo(() => {
    const grouped = new Map<string, ScoreRow[]>();
    for (const row of scoreData) {
      const existing = grouped.get(row.title);
      if (existing) {
        existing.push(row);
      } else {
        grouped.set(row.title, [row]);
      }
    }
    return grouped;
  }, [scoreData]);

  const versionOptions = useMemo(() => {
    if (versionsResponse.length > 0) {
      return sortByOrder(versionsResponse, VERSION_ORDER_MAP, locale);
    }

    return sortByOrder(
      Array.from(
        new Set(scoreData.map((row) => row.version).filter((value): value is string => Boolean(value))),
      ),
      VERSION_ORDER_MAP,
      locale,
    );
  }, [locale, scoreData, versionsResponse]);

  const loadData = useCallback(async (options?: { force?: boolean; throwOnError?: boolean }) => {
    if (!songInfoUrl.trim() || !recordCollectorUrl.trim()) {
      loadedExplorerKeyRef.current = null;
      loadedPlaylogsKeyRef.current = null;
      playlogLoadAbortRef.current?.abort();
      setIsLoading(false);
      setIsPlaylogsLoading(false);
      setScoreRecords([]);
      setPlaylogRecords([]);
      setVersionsResponse([]);
      setPickerVersionOptions([]);
      setPlayerProfile(null);
      setSongMetadata(new Map<string, SongInfoResponse>());
      setLoadingError({ kind: 'translated', key: 'app.missingUrls' });
      setPlaylogLoadingError(null);
      return;
    }

    const requestKey = `${songInfoUrl.trim()}::${recordCollectorUrl.trim()}`;
    if (options?.force) {
      loadedExplorerKeyRef.current = null;
    } else if (loadedExplorerKeyRef.current === requestKey) {
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
      setSongMetadata(payload.songMetadata);
      const availableVersions = filterAvailableVersions(payload.versions ?? []);
      setVersionsResponse(availableVersions.map((version) => version.version_name));
      setPickerVersionOptions(availableVersions);
      setPlayerProfile(payload.playerProfile);
      loadedExplorerKeyRef.current = requestKey;
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }
      loadedExplorerKeyRef.current = null;
      const message = error instanceof Error ? error.message : String(error);
      setLoadingError({ kind: 'message', message });
      setScoreRecords([]);
      setSongMetadata(new Map<string, SongInfoResponse>());
      setVersionsResponse([]);
      setPickerVersionOptions([]);
      setPlayerProfile(null);
      if (options?.throwOnError) {
        throw error instanceof Error ? error : new Error(String(error));
      }
    } finally {
      if (!controller.signal.aborted) {
        setIsLoading(false);
      }
    }
  }, [recordCollectorUrl, songInfoUrl]);

  const loadPlaylogs = useCallback(async (options?: { force?: boolean; throwOnError?: boolean }) => {
    if (!recordCollectorUrl.trim()) {
      loadedPlaylogsKeyRef.current = null;
      playlogLoadAbortRef.current?.abort();
      setIsPlaylogsLoading(false);
      setPlaylogRecords([]);
      setPlaylogLoadingError(null);
      return;
    }

    const requestKey = recordCollectorUrl.trim();
    const previousLoadedPlaylogsKey = loadedPlaylogsKeyRef.current;
    if (options?.force) {
      loadedPlaylogsKeyRef.current = null;
    } else if (loadedPlaylogsKeyRef.current === requestKey) {
      setIsPlaylogsLoading(false);
      setPlaylogLoadingError(null);
      return;
    }

    const shouldPreserveRecords =
      playlogRecordCountRef.current > 0 && previousLoadedPlaylogsKey === requestKey;

    playlogLoadAbortRef.current?.abort();
    const controller = new AbortController();
    playlogLoadAbortRef.current = controller;

    setIsPlaylogsLoading(true);
    setPlaylogLoadingError(null);

    if (!shouldPreserveRecords) {
      setPlaylogRecords([]);
    }

    try {
      const playlogs = await fetchRecentPlaylogs(recordCollectorUrl, controller.signal);

      if (controller.signal.aborted) {
        return;
      }

      setPlaylogRecords(playlogs);
      loadedPlaylogsKeyRef.current = requestKey;
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }
      loadedPlaylogsKeyRef.current = null;
      const message = error instanceof Error ? error.message : String(error);
      setPlaylogLoadingError({ kind: 'message', message });
      setPlaylogRecords([]);
      if (options?.throwOnError) {
        throw error instanceof Error ? error : new Error(String(error));
      }
    } finally {
      if (!controller.signal.aborted) {
        setIsPlaylogsLoading(false);
      }
    }
  }, [recordCollectorUrl]);

  useEffect(() => {
    void loadData();

    return () => {
      loadAbortRef.current?.abort();
      playlogLoadAbortRef.current?.abort();
    };
  }, [loadData]);

  useEffect(() => {
    if (activePage !== 'playlogs') {
      return;
    }

    void loadPlaylogs();
  }, [activePage, loadPlaylogs]);


  useEffect(() => {
    const onHashChange = () => {
      const nextPage = readPageFromHash(window.location.hash);
      if (!recordCollectorUrl.trim() && nextPage !== 'home' && nextPage !== 'setup' && nextPage !== 'settings') {
        setActivePage('home');
        return;
      }
      setActivePage(nextPage);
    };

    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [recordCollectorUrl]);

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
    syncFilter,
    versionSelection,
  ]);

  useEffect(() => {
    const payload: StoredPlaylogFilters = {
      chartFilter: playlogChartFilter,
      difficultyFilter: playlogDifficultyFilter,
      achievementMin: playlogAchievementMin,
      achievementMax: playlogAchievementMax,
      bestOnly: playlogBestOnly,
      newRecordOnly: playlogNewRecordOnly,
    };
    localStorage.setItem(PLAYLOG_FILTERS_STORAGE_KEY, JSON.stringify(payload));
  }, [
    playlogAchievementMax,
    playlogAchievementMin,
    playlogBestOnly,
    playlogChartFilter,
    playlogDifficultyFilter,
    playlogNewRecordOnly,
  ]);

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

  const handleOpenSongDetail = useCallback((target: SongDetailTarget) => {
    setSelectedDetailSongKey(songIdentityKey(target.title, target.genre, target.artist));
  }, []);

  const closeSongDetail = useCallback(() => {
    setSelectedDetailSongKey(null);
  }, []);

  const handleRefreshSongScores = useCallback(async (target: SongDetailTarget) => {
    await refreshSongScores(recordCollectorUrl, target);
    await loadData({ force: true, throwOnError: true });
  }, [loadData, recordCollectorUrl]);

  const handleOpenHistory = useCallback((row: ScoreRow) => {
    setSelectedHistoryKey(row.key);
    void loadPlaylogs();
  }, [loadPlaylogs]);

  const closeHistory = useCallback(() => {
    setSelectedHistoryKey(null);
  }, []);

  const resolvePlaylogScoreRow = useCallback((row: PlaylogRow): ScoreRow | null => {
    if (row.difficulty === null) {
      return null;
    }

    const titleMatches = scoreRowsByTitle.get(row.title) ?? [];
    const chartMatches = titleMatches.filter(
      (scoreRow) => scoreRow.chartType === row.chartType && scoreRow.difficulty === row.difficulty,
    );

    if (chartMatches.length === 1) {
      return chartMatches[0];
    }

    return chartMatches.find((scoreRow) => scoreRow.songKey === row.songKey) ?? null;
  }, [scoreRowsByTitle]);

  const getPlaylogSongDetailTarget = useCallback((row: PlaylogRow): SongDetailTarget | null => {
    const matchedScoreRow = resolvePlaylogScoreRow(row);
    if (matchedScoreRow) {
      return matchedScoreRow;
    }

    if (!row.genre && !row.artist) {
      return null;
    }

    return row;
  }, [resolvePlaylogScoreRow]);

  const handleOpenPlaylogHistory = useCallback((row: PlaylogRow) => {
    const matchedScoreRow = resolvePlaylogScoreRow(row);
    if (!matchedScoreRow) {
      return;
    }

    handleOpenHistory(matchedScoreRow);
  }, [handleOpenHistory, resolvePlaylogScoreRow]);

  const canOpenPlaylogHistory = useCallback((row: PlaylogRow) => {
    return resolvePlaylogScoreRow(row) !== null;
  }, [resolvePlaylogScoreRow]);

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

  const selectedInternalLevelPresets = useMemo(
    () => resolvePresetSelectionFromRange(INTERNAL_LEVEL_PRESETS, internalMin, internalMax),
    [internalMax, internalMin],
  );
  const selectedScoreRankPresets = useMemo(
    () => resolvePresetSelectionFromRange(SCORE_ACHIEVEMENT_PRESETS, achievementMin, achievementMax),
    [achievementMax, achievementMin],
  );
  const filteredFcOptions = FC_FILTER_OPTIONS;
  const filteredSyncOptions = SYNC_FILTER_OPTIONS;

  const handleInternalMinChange = useCallback((value: number) => {
    setInternalLevelSelectionState(null);
    setInternalMin(value);
  }, []);

  const handleInternalMaxChange = useCallback((value: number) => {
    setInternalLevelSelectionState(null);
    setInternalMax(value);
  }, []);

  const handleAchievementMinChange = useCallback((value: number) => {
    setScoreAchievementSelectionState(null);
    setAchievementMin(value);
  }, []);

  const handleAchievementMaxChange = useCallback((value: number) => {
    setScoreAchievementSelectionState(null);
    setAchievementMax(value);
  }, []);

  const handleInternalLevelPresetToggle = useCallback((value: string) => {
    const next = updateDirectionalRangeSelection({
      order: INTERNAL_LEVEL_CONTIGUOUS_PRESET_ORDER,
      currentSelection: selectedInternalLevelPresets,
      currentState: internalLevelSelectionState,
      clicked: value as InternalLevelPresetId,
    });
    const range = getPresetSelectionRange(
      INTERNAL_LEVEL_PRESETS,
      next.selection as InternalLevelPresetId[],
    );
    if (!range) {
      return;
    }

    setInternalLevelSelectionState(
      next.state as DirectionalRangeSelectionState<Exclude<InternalLevelPresetId, 'ALL'>> | null,
    );
    setInternalMin(range.min);
    setInternalMax(range.max);
  }, [internalLevelSelectionState, selectedInternalLevelPresets]);

  const handleScoreRankPresetToggle = useCallback((value: string) => {
    const next = updateDirectionalRangeSelection({
      order: SCORE_ACHIEVEMENT_PRESET_ORDER,
      currentSelection: selectedScoreRankPresets,
      currentState: scoreAchievementSelectionState,
      clicked: value as ScoreAchievementPresetId,
    });
    const range = getPresetSelectionRange(
      SCORE_ACHIEVEMENT_PRESETS,
      next.selection as ScoreAchievementPresetId[],
    );
    if (!range) {
      return;
    }

    setScoreAchievementSelectionState(
      next.state as DirectionalRangeSelectionState<Exclude<ScoreAchievementPresetId, 'ALL'>> | null,
    );
    setAchievementMin(range.min);
    setAchievementMax(range.max);
  }, [scoreAchievementSelectionState, selectedScoreRankPresets]);

  const handleFcFilterToggle = useCallback((value: string) => {
    const next = updateDirectionalRangeSelection({
      order: filteredFcOptions.filter((item) => item !== ALL_FILTER_PRESET_ID) as Exclude<FcFilterOptionId, 'ALL'>[],
      currentSelection: fcFilter,
      currentState: fcSelectionState,
      clicked: value as FcFilterOptionId,
    });
    const selectableOptions = filteredFcOptions.filter((item) => item !== ALL_FILTER_PRESET_ID);
    const nextSelection = next.selection as FcFilterOptionId[];
    const shouldCollapseToAll = selectableOptions.every((item) => nextSelection.includes(item));

    setFcSelectionState(
      shouldCollapseToAll
        ? null
        : next.state as DirectionalRangeSelectionState<Exclude<FcFilterOptionId, 'ALL'>> | null,
    );
    setFcFilter(shouldCollapseToAll ? [ALL_FILTER_PRESET_ID] : nextSelection);
  }, [fcFilter, fcSelectionState, filteredFcOptions]);

  const handleSyncFilterToggle = useCallback((value: string) => {
    const next = updateDirectionalRangeSelection({
      order: filteredSyncOptions.filter((item) => item !== ALL_FILTER_PRESET_ID) as Exclude<SyncFilterOptionId, 'ALL'>[],
      currentSelection: syncFilter,
      currentState: syncSelectionState,
      clicked: value as SyncFilterOptionId,
    });
    const selectableOptions = filteredSyncOptions.filter((item) => item !== ALL_FILTER_PRESET_ID);
    const nextSelection = next.selection as SyncFilterOptionId[];
    const shouldCollapseToAll = selectableOptions.every((item) => nextSelection.includes(item));

    setSyncSelectionState(
      shouldCollapseToAll
        ? null
        : next.state as DirectionalRangeSelectionState<Exclude<SyncFilterOptionId, 'ALL'>> | null,
    );
    setSyncFilter(shouldCollapseToAll ? [ALL_FILTER_PRESET_ID] : nextSelection);
  }, [filteredSyncOptions, syncFilter, syncSelectionState]);

  const handleApplyScoreQuery = useCallback(() => {
    const nextQuery = scoreQueryDraft.trim();
    setScoreQueryDraft(nextQuery);
    setQuery(nextQuery);
  }, [scoreQueryDraft]);

  const handleApplyPlaylogQuery = useCallback(() => {
    const nextQuery = playlogQueryDraft.trim();
    setPlaylogQueryDraft(nextQuery);
    setPlaylogQuery(nextQuery);
  }, [playlogQueryDraft]);

  const handleResetScoreFilters = useCallback(() => {
    setQuery('');
    setScoreQueryDraft('');
    setChartFilter([...CHART_TYPES]);
    setDifficultyFilter([...DIFFICULTIES]);
    setVersionSelection(DEFAULT_SCORE_FILTERS.versionSelection);
    setFcFilter([ALL_FILTER_PRESET_ID]);
    setSyncFilter([ALL_FILTER_PRESET_ID]);
    setAchievementMin(DEFAULT_SCORE_FILTERS.achievementMin);
    setAchievementMax(DEFAULT_SCORE_FILTERS.achievementMax);
    setInternalMin(DEFAULT_SCORE_FILTERS.internalMin);
    setInternalMax(DEFAULT_SCORE_FILTERS.internalMax);
    setDaysMin(DEFAULT_SCORE_FILTERS.daysMin);
    setDaysMax(DEFAULT_SCORE_FILTERS.daysMax);
    setInternalLevelSelectionState(null);
    setScoreAchievementSelectionState(null);
    setFcSelectionState(null);
    setSyncSelectionState(null);
  }, []);

  const filteredScoreRows = useMemo(
    () =>
      buildFilteredScoreRows({
        scoreData,
        locale,
        query,
        chartFilter,
        difficultyFilter,
        versionSelection,
        versionOptions,
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
      locale,
      query,
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
        locale,
        playlogQuery,
        playlogChartFilter,
        playlogDifficultyFilter,
        playlogAchievementMin,
        playlogAchievementMax,
        playlogBestOnly,
        playlogNewRecordOnly,
        playlogSortKey,
        playlogSortDesc,
        playlogDayStartUnix: selectedPlaylogDayStartUnix,
        playlogDayEndUnix: selectedPlaylogDayEndUnix,
      }),
    [
      playlogAchievementMax,
      playlogAchievementMin,
      playlogBestOnly,
      playlogChartFilter,
      playlogData,
      playlogDifficultyFilter,
      playlogNewRecordOnly,
      locale,
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
      (row): row is RatedScoreRow =>
        row.rating !== null && row.version !== null,
    );
    const newRows = classifiedRows
      .filter((row) => latestSet.has(row.version))
      .sort((left, right) => compareRatingPageRows(left, right, locale))
      .slice(0, 15);
    const oldRows = classifiedRows
      .filter((row) => !latestSet.has(row.version))
      .sort((left, right) => compareRatingPageRows(left, right, locale))
      .slice(0, 35);

    const newTotal = newRows.reduce((sum, row) => sum + (toIntegerRating(row.rating) ?? 0), 0);
    const oldTotal = oldRows.reduce((sum, row) => sum + (toIntegerRating(row.rating) ?? 0), 0);

    return {
      ratingTotal: newTotal + oldTotal,
      newRatingTotal: newTotal,
      oldRatingTotal: oldTotal,
      newRatingRows: newRows,
      oldRatingRows: oldRows,
    };
  }, [locale, scoreData, versionOptions]);

  const handleApplySongInfoUrl = () => {
    const next = songInfoUrlDraft.trim();
    if (!next) return;
    setSongInfoUrl(next);
  };

  const handleApplyRecordCollectorUrl = useCallback((url: string) => {
    setRecordCollectorUrl(url);
    setRecordCollectorUrlDraft(url);
  }, []);

  const handleConnectUrl = handleApplyRecordCollectorUrl;

  const handleNavigatePage = useCallback((page: AppPage) => {
    if (page === 'settings' || page === 'home' || page === 'setup') {
      setSongInfoUrlDraft(songInfoUrl);
      setRecordCollectorUrlDraft(recordCollectorUrl);
    }
    const nextHash = page === 'home'
      ? '#home'
      : page === 'setup'
        ? '#setup'
      : page === 'playlogs'
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

  const scoreCountLabel = `${formatNumber(filteredScoreRows.length)}/${formatNumber(scoreData.length)}`;
  const playlogCountLabel = `${formatNumber(filteredPlaylogRows.length)}/${formatNumber(playlogData.length)}`;
  const loadingErrorMessage = loadingError
    ? loadingError.kind === 'translated'
      ? t(loadingError.key, loadingError.variables)
      : loadingError.message
    : null;
  const playlogLoadingErrorMessage = playlogLoadingError
    ? playlogLoadingError.kind === 'translated'
      ? t(playlogLoadingError.key, playlogLoadingError.variables)
      : playlogLoadingError.message
    : null;
  const navItems = useMemo<Array<{ page: AppPage; label: string; Icon: () => JSX.Element }>>(
    () => [
      { page: 'home', label: t('nav.home'), Icon: HomeIcon },
      { page: 'setup', label: t('nav.setup'), Icon: SetupIcon },
      { page: 'scores', label: t('nav.scores'), Icon: ScoresIcon },
      { page: 'rating', label: t('nav.rating'), Icon: RatingIcon },
      { page: 'playlogs', label: t('nav.playlogs'), Icon: PlaylogsIcon },
      { page: 'picker', label: t('nav.picker'), Icon: PickerIcon },
      { page: 'settings', label: t('nav.settings'), Icon: SettingsIcon },
    ],
    [t],
  );
  const activeNavItem = navItems.find((item) => item.page === activePage) ?? navItems[0];
  const ActiveNavIcon = activeNavItem.Icon;
  const mobileNavItems = navItems;
  const totalPlayCount = playerProfile?.total_play_count;
  const playerInlineSummary = playerProfile ? (
    <p className="app-player-summary" title={`${playerProfile.user_name} @ ${typeof totalPlayCount === 'number' ? formatNumber(totalPlayCount) : '-'}`}>
      <span className="app-player-summary-name">{playerProfile.user_name}</span>
      <span className="app-player-summary-separator">@</span>
      <span className="app-player-summary-count">
        {typeof totalPlayCount === 'number'
          ? formatNumber(totalPlayCount)
          : '-'}
      </span>
    </p>
  ) : null;
  const desktopSidebarTopContent = (
    <section className="panel app-sidebar-inline">
      <div className="app-sidebar-header">
        <div className="brand-copy">
          <h1>maistats</h1>
        </div>
        {playerInlineSummary}
      </div>

      <nav className="app-nav app-nav-inline" aria-label={t('nav.primary')}>
        <div className="app-nav-list">
          {navItems.map(({ page, label, Icon }) => (
            <button
              key={page}
              type="button"
              className={activePage === page ? 'active' : ''}
              onClick={() => handleNavigatePage(page)}
              disabled={!recordCollectorUrl.trim() && page !== 'home' && page !== 'setup' && page !== 'settings'}
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
        <div className="app-sidebar-header">
          <div className="brand-copy">
            <h1>maistats</h1>
          </div>
          {playerInlineSummary}
        </div>

        <nav ref={mobileNavRef} className="app-nav" aria-label={t('nav.primary')}>
          <button
            type="button"
            className="app-nav-trigger"
            aria-expanded={isMobileNavOpen}
            aria-label={t('nav.openPages')}
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
            {navItems.map(({ page, label, Icon }) => (
              <button
                key={page}
                type="button"
                className={activePage === page ? 'active' : ''}
                onClick={() => handleNavigatePage(page)}
                disabled={!recordCollectorUrl.trim() && page !== 'home' && page !== 'setup' && page !== 'settings'}
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
                disabled={!recordCollectorUrl.trim() && page !== 'home' && page !== 'setup' && page !== 'settings'}
              >
                <Icon />
                <span>{label}</span>
              </button>
            ))}
          </div>
        </nav>
      </aside>

      <main className="app-main">
        {activePage === 'home' ? (
          <HomePage
            sidebarTopContent={desktopSidebarTopContent}
            onNavigateToSetup={() => handleNavigatePage('setup')}
          />
        ) : activePage === 'setup' ? (
          <SetupGuidePage
            sidebarTopContent={desktopSidebarTopContent}
            recordCollectorUrl={recordCollectorUrl}
            onConnect={handleConnectUrl}
            onNavigateToScores={() => handleNavigatePage('scores')}
          />
        ) : activePage === 'scores' ? (
          <>
            {loadingErrorMessage ? <section className="error-banner">{t('common.error')}: {loadingErrorMessage}</section> : null}

            <ScoreExplorerSection
              sidebarTopContent={desktopSidebarTopContent}
              scoreCountLabel={scoreCountLabel}
              isLoading={isLoading}
              showJackets={showJackets}
              setShowJackets={setShowJackets}
              appliedQuery={query}
              queryDraft={scoreQueryDraft}
              setQueryDraft={setScoreQueryDraft}
              onApplyQuery={handleApplyScoreQuery}
              chartTypes={CHART_TYPES}
              chartFilter={chartFilter}
              setChartFilter={setChartFilter}
              difficulties={DIFFICULTIES}
              difficultyFilter={difficultyFilter}
              setDifficultyFilter={setDifficultyFilter}
              versionOptions={versionOptions}
              versionSelection={versionSelection}
              setVersionSelection={setVersionSelection}
              internalLevelPresetOptions={INTERNAL_LEVEL_PRESETS.map((preset) => preset.label)}
              selectedInternalLevelPresets={selectedInternalLevelPresets}
              onToggleInternalLevelPreset={handleInternalLevelPresetToggle}
              scoreRankOptions={SCORE_ACHIEVEMENT_PRESETS.map((preset) => preset.label)}
              selectedScoreRankPresets={selectedScoreRankPresets}
              onToggleScoreRankPreset={handleScoreRankPresetToggle}
              fcOptions={filteredFcOptions}
              fcFilter={fcFilter}
              onToggleFcFilter={handleFcFilterToggle}
              syncOptions={filteredSyncOptions}
              syncFilter={syncFilter}
              onToggleSyncFilter={handleSyncFilterToggle}
              achievementMin={achievementMin}
              onChangeAchievementMin={handleAchievementMinChange}
              achievementMax={achievementMax}
              onChangeAchievementMax={handleAchievementMaxChange}
              internalMin={internalMin}
              onChangeInternalMin={handleInternalMinChange}
              internalMax={internalMax}
              onChangeInternalMax={handleInternalMaxChange}
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
              onResetFilters={handleResetScoreFilters}
            />
          </>
        ) : activePage === 'playlogs' ? (
          <>
            {loadingErrorMessage ? <section className="error-banner">{t('common.error')}: {loadingErrorMessage}</section> : null}
            {playlogLoadingErrorMessage ? <section className="error-banner">{t('common.error')}: {playlogLoadingErrorMessage}</section> : null}

            <PlaylogExplorerSection
              sidebarTopContent={desktopSidebarTopContent}
              playlogCountLabel={playlogCountLabel}
              isLoading={isPlaylogsLoading}
              showJackets={showJackets}
              setShowJackets={setShowJackets}
              appliedPlaylogQuery={playlogQuery}
              playlogQueryDraft={playlogQueryDraft}
              setPlaylogQueryDraft={setPlaylogQueryDraft}
              onApplyPlaylogQuery={handleApplyPlaylogQuery}
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
              playlogBestOnly={playlogBestOnly}
              setPlaylogBestOnly={setPlaylogBestOnly}
              playlogNewRecordOnly={playlogNewRecordOnly}
              setPlaylogNewRecordOnly={setPlaylogNewRecordOnly}
              isPlaylogDateFilterDisabled={isPlaylogDateFilterDisabled}
              setIsPlaylogDateFilterDisabled={setIsPlaylogDateFilterDisabled}
              selectedPlaylogDayKey={selectedPlaylogDayKey}
              setSelectedPlaylogDayKey={setSelectedPlaylogDayKey}
              playlogDayOptions={playlogDayOptions}
              selectedPlaylogDayCreditCount={selectedPlaylogDayCreditCount}
              selectedPlaylogDaySongCount={selectedPlaylogDaySongCount}
              filteredPlaylogRows={filteredPlaylogRows}
              songInfoUrl={songInfoUrl}
              getSongDetailTarget={getPlaylogSongDetailTarget}
              canOpenHistory={canOpenPlaylogHistory}
              onOpenSongDetail={handleOpenSongDetail}
              onOpenHistory={handleOpenPlaylogHistory}
              playlogSortKey={playlogSortKey}
              playlogSortDesc={playlogSortDesc}
              onSortBy={handlePlaylogSortBy}
            />
          </>
        ) : activePage === 'rating' ? (
          <>
            {loadingErrorMessage ? <section className="error-banner">{t('common.error')}: {loadingErrorMessage}</section> : null}
            <RatingPage
              sidebarTopContent={desktopSidebarTopContent}
              songInfoUrl={songInfoUrl}
              ratingTotal={ratingTotal}
              newRatingTotal={newRatingTotal}
              oldRatingTotal={oldRatingTotal}
              newRows={newRatingRows}
              oldRows={oldRatingRows}
              onOpenSongDetail={handleOpenSongDetail}
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
            languagePreference={languagePreference}
            setLanguagePreference={setLanguagePreference}
            languageLabel={formatLanguageName(language)}
            themePreference={themePreference}
            setThemePreference={setThemePreferenceState}
            songInfoUrlDraft={songInfoUrlDraft}
            setSongInfoUrlDraft={setSongInfoUrlDraft}
            recordCollectorUrlDraft={recordCollectorUrlDraft}
            setRecordCollectorUrlDraft={setRecordCollectorUrlDraft}
            recordCollectorUrl={recordCollectorUrl}
            onApplySongInfoUrl={handleApplySongInfoUrl}
            onApplyRecordCollectorUrl={handleApplyRecordCollectorUrl}
          />
        )}
      </main>

      <SongDetailModal
        selectedDetailTitle={selectedDetailSong?.title ?? null}
        selectedDetailGenre={selectedDetailSong?.genre ?? null}
        selectedDetailArtist={selectedDetailSong?.artist ?? null}
        selectedDetailAliases={selectedDetailSong?.aliases ?? null}
        selectedDetailRows={selectedDetailRows}
        songInfoUrl={songInfoUrl}
        recordCollectorUrl={recordCollectorUrl}
        onRefreshSongScores={handleRefreshSongScores}
        onClose={closeSongDetail}
      />
      <ScoreHistoryModal
        selectedHistoryRow={selectedHistoryRow}
        historyPoints={selectedHistoryPoints}
        isLoading={isPlaylogsLoading}
        loadingErrorMessage={playlogLoadingErrorMessage}
        songInfoUrl={songInfoUrl}
        onClose={closeHistory}
      />
    </div>
  );
}

export default App;
