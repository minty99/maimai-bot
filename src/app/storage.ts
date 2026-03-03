import type { ChartType, DifficultyCategory, FcStatus, ScoreRank, SyncStatus } from '../types';

export interface StoredScoreFilters {
  chartFilter?: ChartType[];
  difficultyFilter?: DifficultyCategory[];
  versionSelection?: string;
  // legacy multi-select key for backward compatibility
  versionFilter?: string[];
  rankFilter?: ScoreRank[];
  fcFilter?: FcStatus[];
  syncFilter?: SyncStatus[];
  includeNoAchievement?: boolean;
  includeNoInternalLevel?: boolean;
  includeNeverPlayed?: boolean;
  achievementMin?: number;
  achievementMax?: number;
  internalMin?: number;
  internalMax?: number;
  daysMin?: number;
  daysMax?: number;
}

export interface StoredPlaylogFilters {
  chartFilter?: ChartType[];
  difficultyFilter?: DifficultyCategory[];
  includeUnknownDiff?: boolean;
  newRecordOnly?: boolean;
  firstPlayOnly?: boolean;
  achievementMin?: number;
  achievementMax?: number;
}

export interface StoredRandomPickerFilters {
  levelStart?: number;
  levelEnd?: number;
  chartTypes?: ChartType[];
  difficultyIndices?: number[];
  includeVersionIndices?: number[] | null;
}

export function readStoredValue(key: string, fallbackValue: string): string {
  const value = localStorage.getItem(key)?.trim();
  if (!value) {
    return fallbackValue;
  }
  return value;
}

export function readStoredJson<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) {
      return null;
    }
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export function coerceArray<T extends string>(value: unknown, allowed: readonly T[]): T[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is T => typeof item === 'string' && allowed.includes(item as T));
}

export function coerceStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is string => typeof item === 'string');
}

export function coerceNumberArray(value: unknown): number[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is number => typeof item === 'number' && Number.isFinite(item));
}

export function coerceBoolean(value: unknown, fallbackValue: boolean): boolean {
  if (typeof value === 'boolean') {
    return value;
  }
  return fallbackValue;
}

export function coerceNumber(value: unknown, fallbackValue: number): number {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  return fallbackValue;
}
