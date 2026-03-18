import { describe, expect, it } from 'vitest';

import { formatNumber, sortByOrder } from './utils';

describe('formatNumber', () => {
  it('formats numbers with the provided locale', () => {
    expect(formatNumber(1234567, 'en-US')).toBe('1,234,567');
  });

  it('returns a dash for null values', () => {
    expect(formatNumber(null, 'en-US')).toBe('-');
  });

  it('throws when locale is omitted', () => {
    expect(() => formatNumber(1234, undefined as never)).toThrow(
      'formatNumber requires an explicit locale',
    );
  });
});

describe('sortByOrder', () => {
  it('applies explicit ordering before locale sorting', () => {
    const orderMap = new Map<string, number>([['MASTER', 0]]);

    expect(sortByOrder(['EXPERT', 'MASTER', 'ADVANCED'], orderMap, 'en-US')).toEqual([
      'MASTER',
      'ADVANCED',
      'EXPERT',
    ]);
  });

  it('uses locale-aware fallback ordering', () => {
    expect(sortByOrder(['z', 'ä'], new Map<string, number>(), 'sv')).toEqual(['z', 'ä']);
  });
});
