import type { SongAliases, SongVersionResponse } from '../types';

export function formatPercent(value: number | null, digits = 4): string {
  if (value === null) {
    return '-';
  }
  return `${value.toFixed(digits)}%`;
}

export function formatRatio(value: number | null): string {
  if (value === null) {
    return '-';
  }
  return `${(value * 100).toFixed(2)}%`;
}

export function formatNumber(value: number | null, locale: string): string;
export function formatNumber(value: number | null, locale?: string): string {
  if (value === null) {
    return '-';
  }
  if (locale === undefined) {
    throw new Error('formatNumber requires an explicit locale');
  }
  return value.toLocaleString(locale);
}

export function includesText(haystack: string, query: string): boolean {
  if (!query.trim()) {
    return true;
  }
  return haystack.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase());
}

export function compareNullableNumber(a: number | null, b: number | null): number {
  if (a === null && b === null) {
    return 0;
  }
  if (a === null) {
    return -1;
  }
  if (b === null) {
    return 1;
  }
  return a - b;
}

function parseDisplayedLevel(level: string | null | undefined): number | null {
  const normalized = level?.trim();
  if (!normalized) {
    return null;
  }

  const match = /^(\d+)(\+|\?)?$/.exec(normalized);
  if (!match) {
    const parsed = Number(normalized);
    return Number.isFinite(parsed) ? parsed : null;
  }

  const base = Number(match[1]);
  if (!Number.isFinite(base)) {
    return null;
  }

  const suffix = match[2] ?? '';
  if (suffix === '+') {
    return base + 0.6;
  }
  if (suffix === '?') {
    return base + 0.7;
  }
  return base;
}

export function compareLevelOrder(
  leftLevel: string | null | undefined,
  leftInternalLevel: number | null,
  rightLevel: string | null | undefined,
  rightInternalLevel: number | null,
): number {
  const leftParsedLevel = parseDisplayedLevel(leftLevel);
  const rightParsedLevel = parseDisplayedLevel(rightLevel);
  const parsedLevelResult = compareNullableNumber(leftParsedLevel, rightParsedLevel);
  if (parsedLevelResult !== 0) {
    return parsedLevelResult;
  }

  return compareNullableNumber(leftInternalLevel, rightInternalLevel);
}

export function sortByOrder<T extends string>(
  values: T[],
  orderMap: Map<string, number>,
  locale: string,
): T[] {
  return [...values].sort((left, right) => {
    const leftOrder = orderMap.get(left);
    const rightOrder = orderMap.get(right);
    if (leftOrder !== undefined && rightOrder !== undefined) {
      return leftOrder - rightOrder;
    }
    if (leftOrder !== undefined) {
      return -1;
    }
    if (rightOrder !== undefined) {
      return 1;
    }
    return left.localeCompare(right, locale);
  });
}

export function filterAvailableVersions(versions: SongVersionResponse[]): SongVersionResponse[] {
  return versions.filter((version) => version.song_count > 0 && version.version_name.trim().length > 0);
}

export function sortIndicator(isActive: boolean, isDesc: boolean): string {
  if (!isActive) {
    return '↕';
  }
  return isDesc ? '▼' : '▲';
}

export function toggleArrayValue<T extends string>(items: T[], value: T): T[] {
  if (items.includes(value)) {
    return items.filter((item) => item !== value);
  }
  return [...items, value];
}

const DIFFICULTY_SHORT_LABELS: Record<string, string> = {
  BASIC: 'BAS',
  ADVANCED: 'ADV',
  EXPERT: 'EXP',
  MASTER: 'MAS',
  'Re:MASTER': 'Re:M',
};

export function formatDifficultyShort(value: string | null | undefined): string {
  if (!value) {
    return '-';
  }
  return DIFFICULTY_SHORT_LABELS[value] ?? value;
}

export function formatVersionLabel(value: string | null | undefined): string {
  if (!value) {
    return '-';
  }
  return value.replace(/^maimaiでらっくす/, 'DX');
}

export function aliasValues(aliases: SongAliases | null | undefined, language: 'en' | 'ko'): string[] {
  const values = aliases?.[language];
  return Array.isArray(values) ? values : [];
}

function formatAliasGroup(label: string, aliases: string[]): string | null {
  if (aliases.length === 0) {
    return null;
  }
  const visible = aliases.slice(0, 2);
  const remaining = aliases.length - visible.length;
  const suffix = remaining > 0 ? ` +${remaining}` : '';
  return `${label}: ${visible.join(', ')}${suffix}`;
}

export function formatAliasSummary(aliases: SongAliases | null | undefined): string | null {
  const groups = [
    formatAliasGroup('EN', aliasValues(aliases, 'en')),
    formatAliasGroup('KO', aliasValues(aliases, 'ko')),
  ].filter((value): value is string => value !== null);

  if (groups.length === 0) {
    return null;
  }

  return groups.join(' | ');
}
