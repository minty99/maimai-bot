export type ChartType = 'STD' | 'DX';

export type DifficultyCategory =
  | 'BASIC'
  | 'ADVANCED'
  | 'EXPERT'
  | 'MASTER'
  | 'Re:MASTER';

export type ScoreRank =
  | 'SSS+'
  | 'SSS'
  | 'SS+'
  | 'SS'
  | 'S+'
  | 'S'
  | 'AAA'
  | 'AA'
  | 'A'
  | 'BBB'
  | 'BB'
  | 'B'
  | 'C'
  | 'D';

export type FcStatus = 'AP+' | 'AP' | 'FC+' | 'FC';

export type SyncStatus = 'FDX+' | 'FDX' | 'FS+' | 'FS' | 'SYNC';

export interface ApiErrorResponse {
  message?: string;
  code?: string;
  maintenance?: boolean;
}

export interface ScoreApiResponse {
  title: string;
  genre: string;
  artist: string;
  chart_type: ChartType;
  diff_category: DifficultyCategory;
  achievement_x10000: number | null;
  rank: ScoreRank | null;
  fc: FcStatus | null;
  sync: SyncStatus | null;
  dx_score: number | null;
  dx_score_max: number | null;
  last_played_at?: string | null;
  play_count?: number | null;
  rating_points?: number | null;
}

export interface SongDetailScoreApiResponse {
  title: string;
  genre: string;
  artist: string;
  chart_type: ChartType;
  diff_category: DifficultyCategory;
  achievement_x10000?: number | null;
  rank?: ScoreRank | null;
  fc?: FcStatus | null;
  sync?: SyncStatus | null;
  dx_score?: number | null;
  dx_score_max?: number | null;
  last_played_at?: string | null;
  play_count?: number | null;
}

export interface SongDetailRow {
  key: string;
  songKey: string;
  title: string;
  genre: string;
  artist: string;
  imageName: string | null;
  chartType: ChartType;
  difficulty: DifficultyCategory;
  level: string | null;
  internalLevel: number | null;
  isInternalLevelEstimated: boolean;
  achievementPercent: number | null;
  rank: ScoreRank | null;
  fc: FcStatus | null;
  sync: SyncStatus | null;
  dxScore: number | null;
  dxScoreMax: number | null;
  lastPlayedAtLabel: string | null;
  playCount: number | null;
  version: string | null;
}

export interface PlayRecordApiResponse {
  played_at_unixtime: number;
  played_at: string | null;
  track: number | null;
  title: string;
  genre?: string | null;
  artist?: string | null;
  chart_type: ChartType;
  diff_category: DifficultyCategory | null;
  achievement_x10000: number | null;
  score_rank: ScoreRank | null;
  fc: FcStatus | null;
  sync: SyncStatus | null;
  dx_score: number | null;
  dx_score_max: number | null;
  credit_id: number | null;
  achievement_new_record: number | null;
  rating_points?: number | null;
}

export interface SongChartRegion {
  jp: boolean;
  intl: boolean;
}

export interface SongSheetResponse {
  chart_type: ChartType;
  difficulty: DifficultyCategory;
  level: string;
  version: string | null;
  internal_level: number | null;
  region: SongChartRegion;
}

export interface SongInfoResponse {
  title: string;
  genre: string;
  artist: string;
  image_name: string | null;
  sheets: SongSheetResponse[];
}

export interface SongInfoListResponse {
  songs: SongInfoResponse[];
}

export interface SongVersionResponse {
  version_index: number;
  version_name: string;
  song_count: number;
}

export interface SongVersionsListResponse {
  versions: SongVersionResponse[];
}

export interface RandomPickerSong {
  title: string;
  genre: string;
  artist: string;
  version: string | null;
  imageName: string | null;
  chartType: ChartType;
  difficulty: DifficultyCategory;
  level: string;
  internalLevel: number | null;
  achievementX10000: number | null;
  rank: ScoreRank | null;
  fc: FcStatus | null;
  sync: SyncStatus | null;
  dxScore: number | null;
  dxScoreMax: number | null;
  lastPlayedAt: string | null;
  playCount: number | null;
  levelSongCount: number | null;
  filteredSongCount: number | null;
}

export interface ScoreRow {
  key: string;
  songKey: string;
  title: string;
  genre: string;
  artist: string;
  chartType: ChartType;
  difficulty: DifficultyCategory;
  achievementX10000: number | null;
  achievementPercent: number | null;
  rank: ScoreRank | null;
  fc: FcStatus | null;
  sync: SyncStatus | null;
  dxScore: number | null;
  dxScoreMax: number | null;
  dxRatio: number | null;
  ratingPoints: number | null;
  level: string | null;
  internalLevel: number | null;
  isInternalLevelEstimated: boolean;
  version: string | null;
  imageName: string | null;
  latestPlayedAtUnix: number | null;
  latestPlayedAtLabel: string | null;
  daysSinceLastPlayed: number | null;
  playCount: number | null;
}

export interface ScoreHistoryPoint {
  key: string;
  playedAtUnix: number;
  playedAtLabel: string | null;
  achievementPercent: number;
}

export interface PlaylogRow {
  key: string;
  songKey: string;
  title: string;
  genre: string;
  artist: string;
  chartType: ChartType;
  difficulty: DifficultyCategory | null;
  level: string | null;
  internalLevel: number | null;
  isInternalLevelEstimated: boolean;
  playedAtUnix: number;
  playedAtLabel: string | null;
  track: number | null;
  achievementX10000: number | null;
  achievementPercent: number | null;
  rank: ScoreRank | null;
  fc: FcStatus | null;
  sync: SyncStatus | null;
  dxScore: number | null;
  dxScoreMax: number | null;
  dxRatio: number | null;
  ratingPoints: number | null;
  creditId: number | null;
  isNewRecord: boolean;
  imageName: string | null;
}
