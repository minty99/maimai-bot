import type { ChartType, DifficultyCategory } from './types';

function normalizeIdentityComponent(value: string): string {
  return value.trim();
}

export function songIdentityKey(title: string, genre: string, artist: string): string {
  return JSON.stringify([
    normalizeIdentityComponent(title),
    normalizeIdentityComponent(genre),
    normalizeIdentityComponent(artist),
  ]);
}

export function chartIdentityKey(
  title: string,
  genre: string,
  artist: string,
  chartType: ChartType,
  difficulty: DifficultyCategory,
): string {
  return JSON.stringify([
    normalizeIdentityComponent(title),
    normalizeIdentityComponent(genre),
    normalizeIdentityComponent(artist),
    chartType,
    difficulty,
  ]);
}
