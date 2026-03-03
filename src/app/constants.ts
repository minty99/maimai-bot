import type { ChartType, DifficultyCategory, FcStatus, ScoreRank, SyncStatus } from '../types';

export const SONG_INFO_STORAGE_KEY = 'maistats.song-info-url';
export const RECORD_STORAGE_KEY = 'maistats.record-url';
export const SCORE_FILTERS_STORAGE_KEY = 'maistats.score-filters';
export const PLAYLOG_FILTERS_STORAGE_KEY = 'maistats.playlog-filters';
export const RANDOM_PICKER_FILTERS_STORAGE_KEY = 'maistats.random-picker-filters';

const ENV_SONG_INFO_URL = import.meta.env.VITE_SONG_INFO_SERVER_URL?.trim();
const ENV_RECORD_COLLECTOR_URL =
  import.meta.env.VITE_RECORD_COLLECTOR_SERVER_URL?.trim();

export const DEFAULT_SONG_INFO_URL =
  ENV_SONG_INFO_URL || 'http://localhost:3001';
export const DEFAULT_RECORD_COLLECTOR_URL =
  ENV_RECORD_COLLECTOR_URL || 'http://localhost:3000';

export const CHART_TYPES: ChartType[] = ['STD', 'DX'];
export const DIFFICULTIES: DifficultyCategory[] = [
  'BASIC',
  'ADVANCED',
  'EXPERT',
  'MASTER',
  'Re:MASTER',
];
export const DIFFICULTY_INDICES = [0, 1, 2, 3, 4] as const;
export const DIFFICULTY_INDEX_LABELS: Record<number, DifficultyCategory> = {
  0: 'BASIC',
  1: 'ADVANCED',
  2: 'EXPERT',
  3: 'MASTER',
  4: 'Re:MASTER',
};
export const RANDOM_PICKER_GAP_OPTIONS = [0, 0.1, 0.2, 0.5] as const;
export const RANDOM_PICKER_LEVEL_STEP = 0.1;
export const RANDOM_PICKER_MIN_LEVEL = 1;
export const RANDOM_PICKER_MAX_LEVEL = 15;
export const RANDOM_PICKER_DEFAULT_LEVEL = 12.5;

export const VERSION_ORDER = [
  'maimai',
  'maimai PLUS',
  'GreeN',
  'GreeN PLUS',
  'ORANGE',
  'ORANGE PLUS',
  'PiNK',
  'PiNK PLUS',
  'MURASAKi',
  'MURASAKi PLUS',
  'MiLK',
  'MiLK PLUS',
  'FiNALE',
  'maimaiでらっくす',
  'maimaiでらっくす PLUS',
  'Splash',
  'Splash PLUS',
  'UNiVERSE',
  'UNiVERSE PLUS',
  'FESTiVAL',
  'FESTiVAL PLUS',
  'BUDDiES',
  'BUDDiES PLUS',
  'PRiSM',
  'PRiSM PLUS',
  'CiRCLE',
] as const;

export const SCORE_RANK_ORDER: ScoreRank[] = [
  'SSS+',
  'SSS',
  'SS+',
  'SS',
  'S+',
  'S',
  'AAA',
  'AA',
  'A',
  'BBB',
  'BB',
  'B',
  'C',
  'D',
];

export const RANK_FILTER_AAA_OR_BELOW_LABEL = '~AAA';
export const AAA_OR_BELOW_RANKS: ScoreRank[] = SCORE_RANK_ORDER.slice(
  SCORE_RANK_ORDER.indexOf('AAA'),
);

export const FC_ORDER: FcStatus[] = ['AP+', 'AP', 'FC+', 'FC'];
export const SYNC_ORDER: SyncStatus[] = ['FDX+', 'FDX', 'FS+', 'FS', 'SYNC'];

export const VERSION_ORDER_MAP: Map<string, number> = new Map(
  VERSION_ORDER.map((value, index) => [value, index]),
);
export const SCORE_RANK_ORDER_MAP: Map<string, number> = new Map(
  SCORE_RANK_ORDER.map((value, index) => [value, index]),
);
export const FC_ORDER_MAP: Map<string, number> = new Map(
  FC_ORDER.map((value, index) => [value, index]),
);
export const SYNC_ORDER_MAP: Map<string, number> = new Map(
  SYNC_ORDER.map((value, index) => [value, index]),
);

export type ActiveTab = 'scores' | 'playlogs';

export type ScoreSortKey =
  | 'title'
  | 'achievement'
  | 'rating'
  | 'internal'
  | 'dxRatio'
  | 'lastPlayed';

export type PlaylogSortKey =
  | 'playedAt'
  | 'achievement'
  | 'rating'
  | 'dxRatio'
  | 'playCount'
  | 'title';
