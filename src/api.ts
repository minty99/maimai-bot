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

  const [ratedScores, playlogs, versionsResult, songMetadata] = await Promise.all([
    getJson<ScoreApiResponse[]>(`${recordBase}/api/scores/rated`, signal),
    getJson<PlayRecordApiResponse[]>(`${recordBase}/api/recent?limit=${RECENT_LIMIT}`, signal),
    getJson<SongVersionsListResponse>(`${songInfoBase}/api/songs/versions`, signal).catch(
      () => null,
    ),
    fetchAllSongMetadata(songInfoBase, signal).catch(() => new Map<string, SongInfoResponse>()),
  ]);

  return {
    ratedScores,
    playlogs,
    songMetadata,
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

export function buildCoverUrl(songInfoBaseUrl: string, imageName: string): string {
  return `${normalizeBaseUrl(songInfoBaseUrl)}/api/cover/${encodeURIComponent(imageName)}`;
}
