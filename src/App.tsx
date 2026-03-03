import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import {
  fetchExplorerPayload,
  fetchSongDetailScores,
} from './api';
import {
  AAA_OR_BELOW_RANKS,
  ActiveTab,
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
  ScoreSortKey,
  SYNC_ORDER,
  VERSION_ORDER_MAP,
} from './app/constants';
import { sortByOrder, toggleArrayValue } from './app/utils';
import {
  coerceArray,
  coerceBoolean,
  coerceNumber,
  coerceStringArray,
  readStoredJson,
  readStoredValue,
  StoredPlaylogFilters,
  StoredScoreFilters,
} from './app/storage';
import {
  buildPlaylogRows,
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
import { ScoreExplorerSection } from './components/ScoreExplorerSection';
import { ServerConnectionModal } from './components/ServerConnectionModal';
import { SongDetailModal } from './components/SongDetailModal';
import type {
  ChartType,
  DifficultyCategory,
  FcStatus,
  PlayRecordApiResponse,
  ScoreApiResponse,
  ScoreRank,
  SongDetailScoreApiResponse,
  SongInfoResponse,
  SyncStatus,
} from './types';

type AppPage = 'explorer' | 'picker';

function readPageFromHash(hash: string): AppPage {
  return hash === '#picker' ? 'picker' : 'explorer';
}

function App() {
  const savedScoreFilters = useMemo(
    () => readStoredJson<StoredScoreFilters>(SCORE_FILTERS_STORAGE_KEY),
    [],
  );
  const savedPlaylogFilters = useMemo(
    () => readStoredJson<StoredPlaylogFilters>(PLAYLOG_FILTERS_STORAGE_KEY),
    [],
  );

  const [activeTab, setActiveTab] = useState<ActiveTab>('scores');
  const [activePage, setActivePage] = useState<AppPage>(() => readPageFromHash(window.location.hash));

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

  const [includeNoAchievement, setIncludeNoAchievement] = useState(() =>
    coerceBoolean(savedScoreFilters?.includeNoAchievement, true),
  );
  const [includeNoInternalLevel, setIncludeNoInternalLevel] = useState(() =>
    coerceBoolean(savedScoreFilters?.includeNoInternalLevel, true),
  );
  const [includeNeverPlayed, setIncludeNeverPlayed] = useState(() =>
    coerceBoolean(savedScoreFilters?.includeNeverPlayed, true),
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

  const [selectedDetailTitle, setSelectedDetailTitle] = useState<string | null>(null);
  const [selectedDetailRows, setSelectedDetailRows] = useState<SongDetailScoreApiResponse[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [isServerModalOpen, setIsServerModalOpen] = useState(false);

  const [playlogQuery, setPlaylogQuery] = useState('');
  const [playlogChartFilter, setPlaylogChartFilter] = useState<ChartType[]>(() => {
    const values = coerceArray(savedPlaylogFilters?.chartFilter, CHART_TYPES);
    return values.length > 0 ? values : [...CHART_TYPES];
  });
  const [playlogDifficultyFilter, setPlaylogDifficultyFilter] = useState<DifficultyCategory[]>(() => {
    const values = coerceArray(savedPlaylogFilters?.difficultyFilter, DIFFICULTIES);
    return values.length > 0 ? values : [...DIFFICULTIES];
  });
  const [playlogIncludeUnknownDiff, setPlaylogIncludeUnknownDiff] = useState(() =>
    coerceBoolean(savedPlaylogFilters?.includeUnknownDiff, true),
  );
  const [playlogNewRecordOnly, setPlaylogNewRecordOnly] = useState(() =>
    coerceBoolean(savedPlaylogFilters?.newRecordOnly, false),
  );
  const [playlogFirstPlayOnly, setPlaylogFirstPlayOnly] = useState(() =>
    coerceBoolean(savedPlaylogFilters?.firstPlayOnly, false),
  );
  const [playlogAchievementMin, setPlaylogAchievementMin] = useState(() =>
    coerceNumber(savedPlaylogFilters?.achievementMin, 0),
  );
  const [playlogAchievementMax, setPlaylogAchievementMax] = useState(() =>
    coerceNumber(savedPlaylogFilters?.achievementMax, 101),
  );
  const [playlogSortKey, setPlaylogSortKey] = useState<PlaylogSortKey>('playedAt');
  const [playlogSortDesc, setPlaylogSortDesc] = useState(true);

  const loadAbortRef = useRef<AbortController | null>(null);
  const detailAbortRef = useRef<AbortController | null>(null);

  const scoreData = useMemo(
    () => buildScoreRows(scoreRecords, songMetadata),
    [scoreRecords, songMetadata],
  );
  const playlogData = useMemo(
    () => buildPlaylogRows(playlogRecords, songMetadata),
    [playlogRecords, songMetadata],
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
    if (activePage !== 'explorer') {
      loadAbortRef.current?.abort();
      setIsLoading(false);
      setLoadingError(null);
      return;
    }

    if (!songInfoUrl.trim() || !recordCollectorUrl.trim()) {
      setIsLoading(false);
      setScoreRecords([]);
      setPlaylogRecords([]);
      setVersionsResponse([]);
      setSongMetadata(new Map<string, SongInfoResponse>());
      setLoadingError('Scores / Playlogs Explorer는 Song Info와 Record Collector URL이 모두 필요합니다.');
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
    } catch (error) {
      if (controller.signal.aborted) {
        return;
      }
      const message = error instanceof Error ? error.message : String(error);
      setLoadingError(message);
      setScoreRecords([]);
      setPlaylogRecords([]);
      setSongMetadata(new Map<string, SongInfoResponse>());
      setVersionsResponse([]);
    } finally {
      if (!controller.signal.aborted) {
        setIsLoading(false);
      }
    }
  }, [activePage, recordCollectorUrl, songInfoUrl]);

  useEffect(() => {
    void loadData();

    return () => {
      loadAbortRef.current?.abort();
      detailAbortRef.current?.abort();
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
      includeNoAchievement,
      includeNoInternalLevel,
      includeNeverPlayed,
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
    includeNeverPlayed,
    includeNoAchievement,
    includeNoInternalLevel,
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
      includeUnknownDiff: playlogIncludeUnknownDiff,
      newRecordOnly: playlogNewRecordOnly,
      firstPlayOnly: playlogFirstPlayOnly,
      achievementMin: playlogAchievementMin,
      achievementMax: playlogAchievementMax,
    };
    localStorage.setItem(PLAYLOG_FILTERS_STORAGE_KEY, JSON.stringify(payload));
  }, [
    playlogAchievementMax,
    playlogAchievementMin,
    playlogChartFilter,
    playlogDifficultyFilter,
    playlogFirstPlayOnly,
    playlogIncludeUnknownDiff,
    playlogNewRecordOnly,
  ]);

  useEffect(() => {
    localStorage.setItem(SONG_INFO_STORAGE_KEY, songInfoUrl);
  }, [songInfoUrl]);

  useEffect(() => {
    localStorage.setItem(RECORD_STORAGE_KEY, recordCollectorUrl);
  }, [recordCollectorUrl]);

  useEffect(() => {
    detailAbortRef.current?.abort();
    setSelectedDetailTitle(null);
    setSelectedDetailRows([]);
    setDetailError(null);
    setDetailLoading(false);
  }, [recordCollectorUrl]);

  const handleOpenSongDetail = useCallback(
    async (title: string) => {
      detailAbortRef.current?.abort();
      const controller = new AbortController();
      detailAbortRef.current = controller;

      setSelectedDetailTitle(title);
      setDetailLoading(true);
      setDetailError(null);
      setSelectedDetailRows([]);

      try {
        const detailRows = await fetchSongDetailScores(
          recordCollectorUrl,
          title,
          controller.signal,
        );
        if (controller.signal.aborted) {
          return;
        }
        setSelectedDetailRows(detailRows);
      } catch (error) {
        if (controller.signal.aborted) {
          return;
        }
        const message = error instanceof Error ? error.message : String(error);
        setDetailError(message);
      } finally {
        if (!controller.signal.aborted) {
          setDetailLoading(false);
        }
      }
    },
    [recordCollectorUrl],
  );

  const closeSongDetail = useCallback(() => {
    detailAbortRef.current?.abort();
    setSelectedDetailTitle(null);
    setSelectedDetailRows([]);
    setDetailError(null);
    setDetailLoading(false);
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
        includeNoAchievement,
        achievementMin,
        achievementMax,
        includeNoInternalLevel,
        internalMin,
        internalMax,
        includeNeverPlayed,
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
      includeNeverPlayed,
      includeNoAchievement,
      includeNoInternalLevel,
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

  const filteredPlaylogRows = useMemo(
    () =>
      buildFilteredPlaylogRows({
        playlogData,
        playlogQuery,
        playlogChartFilter,
        playlogDifficultyFilter,
        playlogIncludeUnknownDiff,
        playlogNewRecordOnly,
        playlogFirstPlayOnly,
        playlogAchievementMin,
        playlogAchievementMax,
        playlogSortKey,
        playlogSortDesc,
      }),
    [
      playlogAchievementMax,
      playlogAchievementMin,
      playlogChartFilter,
      playlogData,
      playlogDifficultyFilter,
      playlogFirstPlayOnly,
      playlogIncludeUnknownDiff,
      playlogNewRecordOnly,
      playlogQuery,
      playlogSortDesc,
      playlogSortKey,
    ],
  );

  const handleApplyUrls = () => {
    const nextSongInfoUrl = songInfoUrlDraft.trim();
    const nextRecordUrl = recordCollectorUrlDraft.trim();

    if (!nextSongInfoUrl) {
      return;
    }

    setSongInfoUrl(nextSongInfoUrl);
    setRecordCollectorUrl(nextRecordUrl);
    setIsServerModalOpen(false);
  };

  const handleNavigatePage = useCallback((page: AppPage) => {
    const nextHash = page === 'picker' ? '#picker' : '#explorer';
    if (window.location.hash !== nextHash) {
      window.location.hash = nextHash;
      return;
    }
    setActivePage(page);
  }, []);

  const openServerModal = useCallback(() => {
    setSongInfoUrlDraft(songInfoUrl);
    setRecordCollectorUrlDraft(recordCollectorUrl);
    setIsServerModalOpen(true);
  }, [recordCollectorUrl, songInfoUrl]);

  const metadataCoverage = `${songMetadata.size.toLocaleString()}곡`;
  const scoreCountLabel = `${filteredScoreRows.length.toLocaleString()}/${scoreData.length.toLocaleString()}`;
  const playlogCountLabel = `${filteredPlaylogRows.length.toLocaleString()}/${playlogData.length.toLocaleString()}`;

  return (
    <div className={activePage === 'picker' ? 'app-shell page-picker' : 'app-shell'}>
      <header className="hero">
        <div className="hero-main">
          <div className="hero-title-row">
            <div>
              <h1>maistats</h1>
            </div>
            <button type="button" className="server-open-button" onClick={openServerModal}>
              Server Connection
            </button>
          </div>
          {activePage === 'explorer' ? (
            <>
              <p>
                점수와 플레이로그를 합쳐서 오래된 곡, 높은 점수 곡, 재도전 우선순위를 자유롭게 정렬/필터링할 수 있는 분석 대시보드입니다.
              </p>
              <span className="meta-note">
                메타데이터: {metadataCoverage}
                {isLoading ? ' (로딩 중...)' : ''}
              </span>
            </>
          ) : null}
        </div>
      </header>

      <section className="tabs">
        <button
          type="button"
          className={activePage === 'explorer' ? 'active' : ''}
          onClick={() => handleNavigatePage('explorer')}
        >
          Explorer
        </button>
        <button
          type="button"
          className={activePage === 'picker' ? 'active' : ''}
          onClick={() => handleNavigatePage('picker')}
        >
          Random Picker
        </button>
      </section>

      {activePage === 'explorer' ? (
        <>
          {loadingError ? <section className="error-banner">에러: {loadingError}</section> : null}

          <section className="tabs">
            <button
              type="button"
              className={activeTab === 'scores' ? 'active' : ''}
              onClick={() => setActiveTab('scores')}
            >
              Scores Explorer
            </button>
            <button
              type="button"
              className={activeTab === 'playlogs' ? 'active' : ''}
              onClick={() => setActiveTab('playlogs')}
            >
              Playlogs Explorer
            </button>
          </section>

          {activeTab === 'scores' ? (
            <ScoreExplorerSection
              scoreCountLabel={scoreCountLabel}
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
              includeNoAchievement={includeNoAchievement}
              setIncludeNoAchievement={setIncludeNoAchievement}
              includeNoInternalLevel={includeNoInternalLevel}
              setIncludeNoInternalLevel={setIncludeNoInternalLevel}
              includeNeverPlayed={includeNeverPlayed}
              setIncludeNeverPlayed={setIncludeNeverPlayed}
              filteredScoreRows={filteredScoreRows}
              songInfoUrl={songInfoUrl}
              onOpenSongDetail={handleOpenSongDetail}
              scoreSortKey={scoreSortKey}
              scoreSortDesc={scoreSortDesc}
              onSortBy={handleScoreSortBy}
            />
          ) : (
            <PlaylogExplorerSection
              playlogCountLabel={playlogCountLabel}
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
              playlogIncludeUnknownDiff={playlogIncludeUnknownDiff}
              setPlaylogIncludeUnknownDiff={setPlaylogIncludeUnknownDiff}
              playlogNewRecordOnly={playlogNewRecordOnly}
              setPlaylogNewRecordOnly={setPlaylogNewRecordOnly}
              playlogFirstPlayOnly={playlogFirstPlayOnly}
              setPlaylogFirstPlayOnly={setPlaylogFirstPlayOnly}
              filteredPlaylogRows={filteredPlaylogRows}
              songInfoUrl={songInfoUrl}
              playlogSortKey={playlogSortKey}
              playlogSortDesc={playlogSortDesc}
              onSortBy={handlePlaylogSortBy}
            />
          )}
        </>
      ) : (
        <RandomPickerPage
          songInfoUrl={songInfoUrl}
          recordCollectorUrl={recordCollectorUrl}
        />
      )}

      <ServerConnectionModal
        isOpen={isServerModalOpen}
        onClose={() => setIsServerModalOpen(false)}
        songInfoUrl={songInfoUrl}
        recordCollectorUrl={recordCollectorUrl}
        songInfoUrlDraft={songInfoUrlDraft}
        setSongInfoUrlDraft={setSongInfoUrlDraft}
        recordCollectorUrlDraft={recordCollectorUrlDraft}
        setRecordCollectorUrlDraft={setRecordCollectorUrlDraft}
        onApply={handleApplyUrls}
      />

      <SongDetailModal
        selectedDetailTitle={selectedDetailTitle}
        selectedDetailRows={selectedDetailRows}
        detailLoading={detailLoading}
        detailError={detailError}
        onClose={closeSongDetail}
      />
    </div>
  );
}

export default App;
