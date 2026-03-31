import type { TranslationKey, TranslationVariables } from './app/i18n';
import { VERSION_ORDER } from './app/constants';
import type {
  ApiErrorResponse,
  CollectorLogsResponse,
  CollectorVersionResponse,
  PlayRecordApiResponse,
  ScoreApiResponse,
  SongDatabaseChartResponse,
  SongDatabaseResponse,
  SongInfoResponse,
  SongVersionResponse,
} from './types';
import { songIdentityKey } from './songIdentity';
import { APP_VERSION, isMinorOrMoreOutdated } from './version';

export class LocalizedApiError extends Error {
  constructor(
    readonly translationKey: TranslationKey,
    readonly variables?: TranslationVariables,
    readonly shouldWrap = false,
  ) {
    super(translationKey);
    this.name = 'LocalizedApiError';
  }
}

export function formatApiErrorMessage(
  error: unknown,
  t: (key: TranslationKey, variables?: TranslationVariables) => string,
): string {
  if (error instanceof LocalizedApiError) {
    return t(error.translationKey, error.variables);
  }

  return error instanceof Error ? error.message : String(error);
}

export interface ExplorerPayload {
  ratedScores: ScoreApiResponse[];
  songMetadata: Map<string, SongInfoResponse>;
  versions: SongVersionResponse[] | null;
  playerProfile: PlayerProfile | null;
}

export type RecordCollectorVersionIssue = 'version_mismatch' | 'invalid_response' | 'unreachable';

export type RecordCollectorVersionStatus =
  | {
    currentVersion: string;
    collectorVersion: string | null;
    isOutdated: false;
    issue: null;
  }
  | {
    currentVersion: string;
    collectorVersion: string;
    isOutdated: true;
    issue: 'version_mismatch';
  }
  | {
    currentVersion: string;
    collectorVersion: string | null;
    isOutdated: true;
    issue: 'invalid_response' | 'unreachable';
  };

const RECENT_LIMIT = 10000;
const COLLECTOR_LOG_LIMIT = 1000;

function normalizeBaseUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) {
    return '';
  }

  try {
    const parsed = new URL(trimmed);
    if (!['http:', 'https:'].includes(parsed.protocol)) {
      return '';
    }

    parsed.pathname = parsed.pathname.replace(/\/+$/, '') || '/';
    parsed.search = '';
    parsed.hash = '';

    return parsed.toString().replace(/\/$/, '');
  } catch {
    return '';
  }
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

async function postJson<TRequest extends object, TResponse>(
  url: string,
  payload: TRequest,
  signal?: AbortSignal,
): Promise<TResponse> {
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
    },
    body: JSON.stringify(payload),
    signal,
  });
  if (!response.ok) {
    const errJson = await safeParseJson<ApiErrorResponse>(response);
    const code = errJson?.code ? `[${errJson.code}] ` : '';
    const message = errJson?.message ?? response.statusText;
    throw new Error(`${code}${message} (HTTP ${response.status})`);
  }

  const data = await safeParseJson<TResponse>(response);
  if (data === null) {
    throw new Error(`Non-JSON response from ${url}`);
  }
  return data;
}

function indexSongMetadata(songs: SongInfoResponse[]): Map<string, SongInfoResponse> {
  const metadata = new Map<string, SongInfoResponse>();

  for (const song of songs) {
    metadata.set(songIdentityKey(song.title, song.genre, song.artist), song);
  }

  return metadata;
}

const CHART_TYPE_MAP: Record<string, 'STD' | 'DX'> = {
  standard: 'STD',
  std: 'STD',
  dx: 'DX',
};

const DIFFICULTY_MAP: Record<string, 'BASIC' | 'ADVANCED' | 'EXPERT' | 'MASTER' | 'Re:MASTER'> = {
  basic: 'BASIC',
  advanced: 'ADVANCED',
  expert: 'EXPERT',
  master: 'MASTER',
  remaster: 'Re:MASTER',
};

function parseChartType(value: string): 'STD' | 'DX' {
  const normalized = value.trim().toLowerCase();
  const chartType = CHART_TYPE_MAP[normalized];
  if (!chartType) {
    throw new Error(`Unsupported chart type in song database: ${value}`);
  }
  return chartType;
}

function parseDifficulty(value: string): 'BASIC' | 'ADVANCED' | 'EXPERT' | 'MASTER' | 'Re:MASTER' {
  const normalized = value.trim().toLowerCase();
  const difficulty = DIFFICULTY_MAP[normalized];
  if (!difficulty) {
    throw new Error(`Unsupported difficulty in song database: ${value}`);
  }
  return difficulty;
}

function parseInternalLevel(value: string | null | undefined): number | null {
  if (typeof value !== 'string' || value.trim().length === 0) {
    return null;
  }

  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function toSongInfoResponse(song: SongDatabaseResponse['songs'][number]): SongInfoResponse {
  return {
    title: song.title,
    genre: song.genre,
    artist: song.artist,
    image_name: song.imageName ?? null,
    aliases: song.aliases ?? {},
    sheets: song.sheets.map((sheet: SongDatabaseChartResponse) => ({
      chart_type: parseChartType(sheet.type),
      difficulty: parseDifficulty(sheet.difficulty),
      level: sheet.level,
      version: sheet.version ?? null,
      internal_level: parseInternalLevel(sheet.internalLevel),
      region: sheet.region,
    })),
  };
}

function deriveSongVersions(songs: SongInfoResponse[]): SongVersionResponse[] {
  const versionsByName = new Map<string, Set<string>>();
  const versionOrderMap = new Map<string, number>(
    VERSION_ORDER.map((name, index) => [name, index]),
  );

  for (const song of songs) {
    for (const sheet of song.sheets) {
      const versionName = sheet.version?.trim();
      if (!sheet.region.intl || !versionName) {
        continue;
      }

      const titles = versionsByName.get(versionName) ?? new Set<string>();
      titles.add(song.title);
      versionsByName.set(versionName, titles);
    }
  }

  return Array.from(versionsByName.entries())
    .map(([versionName, titles]) => ({
      version_index: versionOrderMap.get(versionName) ?? Number.MAX_SAFE_INTEGER,
      version_name: versionName,
      song_count: titles.size,
    }))
    .sort((left, right) => {
      if (left.version_index !== right.version_index) {
        return left.version_index - right.version_index;
      }
      return left.version_name.localeCompare(right.version_name);
    });
}

async function fetchSongDatabase(
  songDatabaseBaseUrl: string,
  signal?: AbortSignal,
): Promise<SongDatabaseResponse> {
  return getJson<SongDatabaseResponse>(
    `${normalizeBaseUrl(songDatabaseBaseUrl)}/data.json`,
    signal,
  );
}

export async function fetchAllSongMetadata(
  songDatabaseBaseUrl: string,
  signal?: AbortSignal,
): Promise<Map<string, SongInfoResponse>> {
  const songDatabaseBase = normalizeBaseUrl(songDatabaseBaseUrl);
  if (!songDatabaseBase) {
    return new Map();
  }

  const response = await fetchSongDatabase(songDatabaseBase, signal);
  return indexSongMetadata(response.songs.map(toSongInfoResponse));
}

export async function fetchExplorerPayload(
  songDatabaseBaseUrl: string,
  recordCollectorBaseUrl: string,
  signal?: AbortSignal,
): Promise<ExplorerPayload> {
  const songDatabaseBase = normalizeBaseUrl(songDatabaseBaseUrl);
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl);

  const [ratedScores, songDatabase, playerProfile] = await Promise.all([
    getJson<ScoreApiResponse[]>(`${recordBase}/api/scores/rated`, signal),
    songDatabaseBase ? fetchSongDatabase(songDatabaseBase, signal).catch(() => null) : null,
    fetchPlayerProfile(recordBase, signal),
  ]);
  const songs = songDatabase?.songs.map(toSongInfoResponse) ?? [];

  return {
    ratedScores,
    songMetadata: indexSongMetadata(songs),
    versions: songDatabase ? deriveSongVersions(songs) : null,
    playerProfile,
  };
}

export async function fetchRecentPlaylogs(
  recordCollectorBaseUrl: string,
  signal?: AbortSignal,
): Promise<PlayRecordApiResponse[]> {
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl);
  if (!recordBase) {
    return [];
  }

  return getJson<PlayRecordApiResponse[]>(`${recordBase}/api/recent?limit=${RECENT_LIMIT}`, signal);
}

export async function fetchCollectorLogs(
  recordCollectorBaseUrl: string,
  signal?: AbortSignal,
): Promise<CollectorLogsResponse> {
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl);
  if (!recordBase) {
    throw new LocalizedApiError('api.recordCollectorRequired');
  }

  return getJson<CollectorLogsResponse>(
    `${recordBase}/api/logs?limit=${COLLECTOR_LOG_LIMIT}`,
    signal,
  );
}

export async function fetchSongVersions(
  songDatabaseBaseUrl: string,
  signal?: AbortSignal,
): Promise<SongVersionResponse[]> {
  const songDatabaseBase = normalizeBaseUrl(songDatabaseBaseUrl);
  if (!songDatabaseBase) {
    return [];
  }

  const response = await fetchSongDatabase(songDatabaseBase, signal);
  return deriveSongVersions(response.songs.map(toSongInfoResponse));
}

export function buildCoverUrl(songDatabaseBaseUrl: string, imageName: string): string {
  return `${normalizeBaseUrl(songDatabaseBaseUrl)}/cover/${encodeURIComponent(imageName)}`;
}

export interface PlayerProfile {
  user_name: string;
  rating: number;
  current_version_play_count: number;
  total_play_count: number;
}

export async function fetchPlayerProfile(
  baseUrl: string,
  signal?: AbortSignal,
): Promise<PlayerProfile | null> {
  const base = normalizeBaseUrl(baseUrl);
  if (!base) return null;
  try {
    return await getJson<PlayerProfile>(`${base}/api/player`, signal);
  } catch {
    return null;
  }
}

export function describeRecordCollectorVersionStatus(
  status: RecordCollectorVersionStatus | null,
): { translationKey: TranslationKey; variables?: TranslationVariables } | null {
  if (!status?.isOutdated) {
    return null;
  }

  switch (status.issue) {
    case 'version_mismatch':
      return {
        translationKey: 'recordCollector.version.outdated',
        variables: {
          currentVersion: status.currentVersion,
          collectorVersion: status.collectorVersion,
        },
      };
    case 'invalid_response':
      return {
        translationKey: 'recordCollector.version.invalid',
        variables: {
          currentVersion: status.currentVersion,
          collectorVersion: status.collectorVersion ?? 'unknown',
        },
      };
    case 'unreachable':
      return {
        translationKey: 'recordCollector.version.unreachable',
        variables: {
          currentVersion: status.currentVersion,
        },
      };
    default: {
      const exhaustive: never = status;
      return exhaustive;
    }
  }
}

function compatibleVersionStatus(collectorVersion: string): RecordCollectorVersionStatus {
  return {
    currentVersion: APP_VERSION,
    collectorVersion,
    isOutdated: false,
    issue: null,
  };
}

function versionMismatchStatus(collectorVersion: string): RecordCollectorVersionStatus {
  return {
    currentVersion: APP_VERSION,
    collectorVersion,
    isOutdated: true,
    issue: 'version_mismatch',
  };
}

function outdatedVersionStatus(
  issue: Exclude<RecordCollectorVersionIssue, 'version_mismatch'>,
  collectorVersion: string | null,
): RecordCollectorVersionStatus {
  return {
    currentVersion: APP_VERSION,
    collectorVersion,
    isOutdated: true,
    issue,
  };
}

export async function fetchRecordCollectorVersionStatus(
  baseUrl: string,
  signal?: AbortSignal,
): Promise<RecordCollectorVersionStatus | null> {
  const base = normalizeBaseUrl(baseUrl);
  if (!base) {
    return baseUrl.trim() ? outdatedVersionStatus('unreachable', null) : null;
  }

  try {
    const httpResponse = await fetch(`${base}/api/version`, { signal });
    if (!httpResponse.ok) {
      throw new Error(`HTTP ${httpResponse.status}`);
    }

    const response = await safeParseJson<CollectorVersionResponse>(httpResponse);
    if (!response || typeof response.version !== 'string') {
      return outdatedVersionStatus('invalid_response', null);
    }

    const outdated = isMinorOrMoreOutdated(APP_VERSION, response.version);

    if (outdated === null) {
      return outdatedVersionStatus('invalid_response', response.version);
    }

    return outdated
      ? versionMismatchStatus(response.version)
      : compatibleVersionStatus(response.version);
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw error;
    }
    return outdatedVersionStatus('unreachable', null);
  }
}

export async function checkRecordCollectorHealth(
  baseUrl: string,
  signal?: AbortSignal,
): Promise<PlayerProfile> {
  const base = normalizeBaseUrl(baseUrl);
  if (!base) {
    throw new LocalizedApiError('api.enterUrl');
  }

  const healthResp = await fetch(`${base}/health/ready`, { signal });
  if (!healthResp.ok) {
    throw new LocalizedApiError('api.connectionFailed', { status: healthResp.status }, true);
  }

  return getJson<PlayerProfile>(`${base}/api/player`, signal);
}

export interface RefreshSongScoresPayload {
  title: string;
  genre: string;
  artist: string;
}

export interface RefreshSongScoresResponse {
  detail_pages_refreshed: number;
  rows_written: number;
}

export async function refreshSongScores(
  baseUrl: string,
  payload: RefreshSongScoresPayload,
  signal?: AbortSignal,
): Promise<RefreshSongScoresResponse> {
  const base = normalizeBaseUrl(baseUrl);
  if (!base) {
    throw new LocalizedApiError('api.recordCollectorRequired');
  }

  return postJson<RefreshSongScoresPayload, RefreshSongScoresResponse>(
    `${base}/api/scores/refresh`,
    payload,
    signal,
  );
}
