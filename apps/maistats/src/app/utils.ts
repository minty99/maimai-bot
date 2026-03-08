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

export function formatNumber(value: number | null): string {
  if (value === null) {
    return '-';
  }
  return value.toLocaleString();
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

export function sortByOrder<T extends string>(values: T[], orderMap: Map<string, number>): T[] {
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
    return left.localeCompare(right, 'ko');
  });
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
