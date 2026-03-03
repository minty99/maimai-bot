import type {
  ApiErrorResponse,
  PlayRecordApiResponse,
  RandomPickerSong,
  RandomSongApiResponse,
  ScoreApiResponse,
  SongDetailScoreApiResponse,
  SongInfoResponse,
  SongVersionResponse,
  SongVersionsListResponse,
} from './types';

export interface ExplorerPayload {
  ratedScores: ScoreApiResponse[];
  playlogs: PlayRecordApiResponse[];
  versions: SongVersionsListResponse | null;
}

const RECENT_LIMIT = 10000;

function normalizeBaseUrl(url: string): string {
  return url.trim().replace(/\/+$/, '');
}

async function safeParseJson<T>(response: Response): Promise<T | null> {
  const contentType = response.headers.get('content-type') ?? '';
  if (!contentType.includes('application/json')) {
    return null;
  }
  try {
    return (await response.json()) as T;
  } catch {
    return null;
  }
}

async function getJson<T>(url: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(url, { signal });
  if (!response.ok) {
    const errJson = await safeParseJson<ApiErrorResponse>(response);
    const code = errJson?.code ? `[${errJson.code}] ` : '';
    const message = errJson?.message ?? response.statusText;
    throw new Error(`${code}${message} (HTTP ${response.status})`);
  }

  const data = await safeParseJson<T>(response);
  if (data === null) {
    throw new Error(`Non-JSON response from ${url}`);
  }
  return data;
}

export async function fetchExplorerPayload(
  songInfoBaseUrl: string,
  recordCollectorBaseUrl: string,
  signal?: AbortSignal,
): Promise<ExplorerPayload> {
  const songInfoBase = normalizeBaseUrl(songInfoBaseUrl);
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl);

  const [ratedScores, playlogs, versionsResult] = await Promise.all([
    getJson<ScoreApiResponse[]>(`${recordBase}/api/scores/rated`, signal),
    getJson<PlayRecordApiResponse[]>(`${recordBase}/api/recent?limit=${RECENT_LIMIT}`, signal),
    getJson<SongVersionsListResponse>(`${songInfoBase}/api/songs/versions`, signal).catch(
      () => null,
    ),
  ]);

  return {
    ratedScores,
    playlogs,
    versions: versionsResult,
  };
}

export async function fetchSongVersions(
  songInfoBaseUrl: string,
  signal?: AbortSignal,
): Promise<SongVersionResponse[]> {
  const songInfoBase = normalizeBaseUrl(songInfoBaseUrl);
  if (!songInfoBase) {
    return [];
  }

  const response = await getJson<SongVersionsListResponse>(
    `${songInfoBase}/api/songs/versions`,
    signal,
  );
  return response.versions;
}

interface MetadataProgress {
  done: number;
  total: number;
}

interface FetchSongMetadataBatchParams {
  songInfoBaseUrl: string;
  titles: string[];
  concurrency?: number;
  signal?: AbortSignal;
  onProgress?: (progress: MetadataProgress) => void;
}

export async function fetchSongMetadataBatch({
  songInfoBaseUrl,
  titles,
  concurrency = 8,
  signal,
  onProgress,
}: FetchSongMetadataBatchParams): Promise<Map<string, SongInfoResponse>> {
  const songInfoBase = normalizeBaseUrl(songInfoBaseUrl);
  const dedupedTitles = Array.from(new Set(titles));
  const total = dedupedTitles.length;

  if (total === 0) {
    return new Map();
  }

  let index = 0;
  let done = 0;
  const metadata = new Map<string, SongInfoResponse>();

  const workerCount = Math.max(1, Math.min(concurrency, total));

  const worker = async () => {
    while (true) {
      if (signal?.aborted) {
        return;
      }

      const currentIndex = index;
      index += 1;
      if (currentIndex >= total) {
        return;
      }

      const title = dedupedTitles[currentIndex];
      try {
        const encoded = encodeURIComponent(title);
        const info = await getJson<SongInfoResponse>(
          `${songInfoBase}/api/songs/by-title/${encoded}`,
          signal,
        );
        metadata.set(normalizeTitleKey(info.title), info);
      } catch {
        // Ignore unresolved songs; score/playlog rows still render without metadata.
      } finally {
        done += 1;
        onProgress?.({ done, total });
      }
    }
  };

  const workers = Array.from({ length: workerCount }, () => worker());
  await Promise.all(workers);

  return metadata;
}

export function normalizeTitleKey(title: string): string {
  return title.trim().toLocaleLowerCase();
}

export function buildCoverUrl(songInfoBaseUrl: string, imageName: string): string {
  return `${normalizeBaseUrl(songInfoBaseUrl)}/api/cover/${encodeURIComponent(imageName)}`;
}

export async function fetchSongDetailScores(
  recordCollectorBaseUrl: string,
  title: string,
  signal?: AbortSignal,
): Promise<SongDetailScoreApiResponse[]> {
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl);
  const encodedTitle = encodeURIComponent(title);
  return getJson<SongDetailScoreApiResponse[]>(
    `${recordBase}/api/scores/detail/${encodedTitle}`,
    signal,
  );
}

interface FetchRandomPickerSongParams {
  songInfoBaseUrl: string;
  recordCollectorBaseUrl?: string;
  minLevel: number;
  maxLevel: number;
  chartTypes: string[];
  difficultyIndices: number[];
  includeVersionIndices: number[] | null;
  signal?: AbortSignal;
}

function toLevelQueryValue(value: number): string {
  const normalized = Math.round(value * 10) / 10;
  return normalized.toFixed(1);
}

function asNullableInt(value: number | null | undefined): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? Math.trunc(value) : null;
}

export async function fetchRandomPickerSong({
  songInfoBaseUrl,
  recordCollectorBaseUrl,
  minLevel,
  maxLevel,
  chartTypes,
  difficultyIndices,
  includeVersionIndices,
  signal,
}: FetchRandomPickerSongParams): Promise<RandomPickerSong> {
  const songInfoBase = normalizeBaseUrl(songInfoBaseUrl);
  if (!songInfoBase) {
    throw new Error('Song Info URL이 비어 있습니다.');
  }

  const params = new URLSearchParams({
    min_level: toLevelQueryValue(minLevel),
    max_level: toLevelQueryValue(maxLevel),
  });

  if (chartTypes.length > 0) {
    params.set('chart_types', [...chartTypes].sort().join(','));
  }
  if (difficultyIndices.length > 0) {
    params.set('include_difficulties', [...difficultyIndices].sort((a, b) => a - b).join(','));
  }
  if (includeVersionIndices !== null && includeVersionIndices.length > 0) {
    params.set('include_versions', [...includeVersionIndices].sort((a, b) => a - b).join(','));
  }

  const song = await getJson<RandomSongApiResponse>(
    `${songInfoBase}/api/songs/random?${params.toString()}`,
    signal,
  );

  if (song.sheets.length === 0) {
    throw new Error('선택된 곡에 sheet 정보가 없습니다.');
  }

  const inRangeSheets = song.sheets.filter((sheet) => {
    if (sheet.internal_level === null) {
      return false;
    }
    return sheet.internal_level >= minLevel && sheet.internal_level <= maxLevel;
  });
  const pool = inRangeSheets.length > 0 ? inRangeSheets : song.sheets;
  const selectedSheet = pool[Math.floor(Math.random() * pool.length)];

  let detailRow: SongDetailScoreApiResponse | undefined;
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl ?? '');
  if (recordBase) {
    try {
      const encodedTitle = encodeURIComponent(song.title);
      const detailRows = await getJson<SongDetailScoreApiResponse[]>(
        `${recordBase}/api/scores/detail/${encodedTitle}`,
        signal,
      );
      detailRow = detailRows.find(
        (row) =>
          row.chart_type === selectedSheet.chart_type &&
          row.diff_category === selectedSheet.difficulty,
      );
    } catch {
      detailRow = undefined;
    }
  }

  return {
    title: song.title,
    version: selectedSheet.version ?? song.version,
    imageName: song.image_name,
    chartType: selectedSheet.chart_type,
    difficulty: selectedSheet.difficulty,
    level: selectedSheet.level,
    internalLevel: selectedSheet.internal_level,
    userLevel: selectedSheet.user_level,
    achievementX10000: detailRow?.achievement_x10000 ?? null,
    rank: detailRow?.rank ?? null,
    fc: detailRow?.fc ?? null,
    sync: detailRow?.sync ?? null,
    dxScore: detailRow?.dx_score ?? null,
    dxScoreMax: detailRow?.dx_score_max ?? null,
    lastPlayedAt: detailRow?.last_played_at ?? null,
    playCount: asNullableInt(detailRow?.play_count),
    levelSongCount: asNullableInt(song.selection_stats?.level_song_count),
    filteredSongCount: asNullableInt(song.selection_stats?.filtered_song_count),
  };
}
