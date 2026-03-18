import { describe, expect, it } from 'vitest';

import { buildFilteredPlaylogRows } from './filtering';
import type { PlaylogRow } from '../types';

function buildPlaylogRow(key: string): PlaylogRow {
  return {
    key,
    songKey: 'song-1',
    title: 'Song',
    genre: 'Genre',
    artist: 'Artist',
    aliases: {},
    chartType: 'DX',
    difficulty: 'MASTER',
    level: '14+',
    internalLevel: 14.7,
    isInternalLevelEstimated: false,
    playedAtUnix: 100,
    playedAtLabel: null,
    track: 1,
    achievementX10000: 1005000,
    achievementPercent: 100.5,
    rank: 'SSS',
    fc: null,
    sync: null,
    dxScore: 1000,
    dxScoreMax: 1200,
    dxRatio: 0.8,
    rating: 15,
    creditId: 1,
    isNewRecord: true,
    imageName: null,
  };
}

describe('buildFilteredPlaylogRows', () => {
  it('uses a locale-independent tiebreaker when best-only is enabled', () => {
    const rows = buildFilteredPlaylogRows({
      playlogData: [buildPlaylogRow('100-1'), buildPlaylogRow('100-2')],
      locale: 'ko-KR',
      playlogQuery: '',
      playlogChartFilter: ['DX'],
      playlogDifficultyFilter: ['MASTER'],
      playlogAchievementMin: 0,
      playlogAchievementMax: 101,
      playlogBestOnly: true,
      playlogNewRecordOnly: false,
      playlogSortKey: 'playedAt',
      playlogSortDesc: true,
      playlogDayStartUnix: null,
      playlogDayEndUnix: null,
    });

    expect(rows).toHaveLength(1);
    expect(rows[0]?.key).toBe('100-2');
  });
});
