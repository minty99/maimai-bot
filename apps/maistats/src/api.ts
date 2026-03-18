import type {
  ApiErrorResponse,
  PlayRecordApiResponse,
  ScoreApiResponse,
  SongInfoListResponse,
  SongInfoResponse,
  SongVersionResponse,
  SongVersionsListResponse,
} from './types';
import { songIdentityKey } from './songIdentity';

export interface ExplorerPayload {
  ratedScores: ScoreApiResponse[];
  playlogs: PlayRecordApiResponse[];
  songMetadata: Map<string, SongInfoResponse>;
  versions: SongVersionsListResponse | null;
  playerProfile: PlayerProfile | null;
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

export async function fetchAllSongMetadata(
  songInfoBaseUrl: string,
  signal?: AbortSignal,
): Promise<Map<string, SongInfoResponse>> {
  const songInfoBase = normalizeBaseUrl(songInfoBaseUrl);
  if (!songInfoBase) {
    return new Map();
  }

  const response = await getJson<SongInfoListResponse>(`${songInfoBase}/api/songs`, signal);
  return indexSongMetadata(response.songs);
}

export async function fetchExplorerPayload(
  songInfoBaseUrl: string,
  recordCollectorBaseUrl: string,
  signal?: AbortSignal,
): Promise<ExplorerPayload> {
  const songInfoBase = normalizeBaseUrl(songInfoBaseUrl);
  const recordBase = normalizeBaseUrl(recordCollectorBaseUrl);

  const [ratedScores, playlogs, versionsResult, songMetadata, playerProfile] = await Promise.all([
    getJson<ScoreApiResponse[]>(`${recordBase}/api/scores/rated`, signal),
    getJson<PlayRecordApiResponse[]>(`${recordBase}/api/recent?limit=${RECENT_LIMIT}`, signal),
    getJson<SongVersionsListResponse>(`${songInfoBase}/api/songs/versions`, signal).catch(
      () => null,
    ),
    fetchAllSongMetadata(songInfoBase, signal).catch(() => new Map<string, SongInfoResponse>()),
    fetchPlayerProfile(recordBase, signal),
  ]);

  return {
    ratedScores,
    playlogs,
    songMetadata,
    versions: versionsResult,
    playerProfile,
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

export function buildCoverUrl(songInfoBaseUrl: string, imageName: string): string {
  return `${normalizeBaseUrl(songInfoBaseUrl)}/api/cover/${encodeURIComponent(imageName)}`;
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

export async function checkRecordCollectorHealth(
  baseUrl: string,
  signal?: AbortSignal,
): Promise<PlayerProfile> {
  const base = normalizeBaseUrl(baseUrl);
  if (!base) {
    throw new Error('URL을 입력해주세요.');
  }

  const healthResp = await fetch(`${base}/health/ready`, { signal });
  if (!healthResp.ok) {
    throw new Error(`서버에 연결할 수 없습니다. (HTTP ${healthResp.status})`);
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
    throw new Error('Record Collector URL이 비어 있습니다.');
  }

  return postJson<RefreshSongScoresPayload, RefreshSongScoresResponse>(
    `${base}/api/scores/refresh`,
    payload,
    signal,
  );
}
