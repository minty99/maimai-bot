import { describe, expect, it } from 'vitest';

import { filterAvailableVersions, formatNumber, sortByOrder } from './utils';

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

describe('filterAvailableVersions', () => {
  it('drops empty and zero-song versions from API payloads', () => {
    expect(
      filterAvailableVersions([
        { version_index: 24, version_name: 'PRiSM PLUS', song_count: 12 },
        { version_index: 25, version_name: 'CiRCLE', song_count: 18 },
        { version_index: 26, version_name: 'CiRCLE PLUS', song_count: 0 },
        { version_index: 27, version_name: '   ', song_count: 3 },
      ]),
    ).toEqual([
      { version_index: 24, version_name: 'PRiSM PLUS', song_count: 12 },
      { version_index: 25, version_name: 'CiRCLE', song_count: 18 },
    ]);
  });
});
