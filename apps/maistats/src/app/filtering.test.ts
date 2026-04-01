import { describe, expect, it } from 'vitest';

import { buildFilteredPlaylogRows, buildFilteredScoreRows } from './filtering';
import { DEFAULT_SCORE_FILTERS } from './scoreFilterPresets';
import type { PlaylogRow, ScoreRow } from '../types';

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

function buildScoreRow(key: string, overrides: Partial<ScoreRow> = {}): ScoreRow {
  return {
    key,
    songKey: key,
    title: `Song ${key}`,
    genre: 'Genre',
    artist: 'Artist',
    aliases: {},
    chartType: 'DX',
    difficulty: 'MASTER',
    achievementX10000: 1005000,
    achievementPercent: 100.5,
    rank: 'SSS',
    fc: null,
    sync: null,
    dxScore: 1000,
    dxScoreMax: 1200,
    dxRatio: 0.8,
    rating: 15,
    level: '14+',
    internalLevel: 14.7,
    isInternalLevelEstimated: false,
    version: 'BUDDiES',
    imageName: null,
    latestPlayedAtUnix: 100,
    latestPlayedAtLabel: null,
    daysSinceLastPlayed: 1,
    playCount: 1,
    ...overrides,
  };
}

describe('buildFilteredScoreRows', () => {
  it('keeps rows without play records when played-only is disabled', () => {
    const rows = buildFilteredScoreRows({
      scoreData: [
        buildScoreRow('played'),
        buildScoreRow('unplayed', {
          achievementX10000: null,
          achievementPercent: null,
          rank: null,
          dxScore: null,
          dxScoreMax: null,
          dxRatio: null,
          rating: null,
          latestPlayedAtUnix: null,
          latestPlayedAtLabel: null,
          daysSinceLastPlayed: null,
          playCount: null,
        }),
      ],
      locale: 'ko-KR',
      query: '',
      chartFilter: ['DX'],
      difficultyFilter: ['MASTER'],
      versionSelection: 'ALL',
      playedOnly: DEFAULT_SCORE_FILTERS.playedOnly,
      versionOptions: [],
      fcFilter: [],
      syncFilter: [],
      achievementMin: 0,
      achievementMax: 101,
      internalMin: 1,
      internalMax: 15.5,
      daysMin: 0,
      daysMax: 2000,
      scoreSortKey: 'title',
      scoreSortDesc: false,
    });

    expect(rows).toHaveLength(2);
  });

  it('filters out rows without play records when played-only is enabled', () => {
    const rows = buildFilteredScoreRows({
      scoreData: [
        buildScoreRow('played'),
        buildScoreRow('unplayed', {
          achievementX10000: null,
          achievementPercent: null,
          rank: null,
          dxScore: null,
          dxScoreMax: null,
          dxRatio: null,
          rating: null,
          latestPlayedAtUnix: null,
          latestPlayedAtLabel: null,
          daysSinceLastPlayed: null,
          playCount: null,
        }),
      ],
      locale: 'ko-KR',
      query: '',
      chartFilter: ['DX'],
      difficultyFilter: ['MASTER'],
      versionSelection: 'ALL',
      playedOnly: true,
      versionOptions: [],
      fcFilter: [],
      syncFilter: [],
      achievementMin: 0,
      achievementMax: 101,
      internalMin: 1,
      internalMax: 15.5,
      daysMin: 0,
      daysMax: 2000,
      scoreSortKey: 'title',
      scoreSortDesc: false,
    });

    expect(rows).toHaveLength(1);
    expect(rows[0]?.key).toBe('played');
  });

  it('sorts score rows by level order', () => {
    const rows = buildFilteredScoreRows({
      scoreData: [
        buildScoreRow('high', { level: '14', internalLevel: 14.0 }),
        buildScoreRow('low', { level: '13', internalLevel: 13.0 }),
        buildScoreRow('mid', { level: '13+', internalLevel: 13.7 }),
      ],
      locale: 'ko-KR',
      query: '',
      chartFilter: ['DX'],
      difficultyFilter: ['MASTER'],
      versionSelection: 'ALL',
      playedOnly: false,
      versionOptions: [],
      fcFilter: [],
      syncFilter: [],
      achievementMin: 0,
      achievementMax: 101,
      internalMin: 1,
      internalMax: 15.5,
      daysMin: 0,
      daysMax: 2000,
      scoreSortKey: 'level',
      scoreSortDesc: false,
    });

    expect(rows.map((row) => row.key)).toEqual(['low', 'mid', 'high']);
  });
});

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

  it('sorts playlog rows by level order', () => {
    const rows = buildFilteredPlaylogRows({
      playlogData: [
        buildPlaylogRow('high'),
        buildPlaylogRow('low'),
        buildPlaylogRow('mid'),
      ].map((row) => {
        if (row.key === 'high') {
          return { ...row, level: '14', internalLevel: 14.0 };
        }
        if (row.key === 'mid') {
          return { ...row, level: '13+', internalLevel: 13.7 };
        }
        return { ...row, level: '13', internalLevel: 13.0 };
      }),
      locale: 'ko-KR',
      playlogQuery: '',
      playlogChartFilter: ['DX'],
      playlogDifficultyFilter: ['MASTER'],
      playlogAchievementMin: 0,
      playlogAchievementMax: 101,
      playlogBestOnly: false,
      playlogNewRecordOnly: false,
      playlogSortKey: 'level',
      playlogSortDesc: false,
      playlogDayStartUnix: null,
      playlogDayEndUnix: null,
    });

    expect(rows.map((row) => row.key)).toEqual(['low', 'mid', 'high']);
  });
});
