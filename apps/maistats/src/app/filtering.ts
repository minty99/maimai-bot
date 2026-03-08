import { FC_ORDER_MAP, SCORE_RANK_ORDER_MAP, ScoreSortKey, SYNC_ORDER_MAP, PlaylogSortKey } from './constants';
import { compareNullableNumber, includesText, sortByOrder } from './utils';
import type { FcStatus, PlaylogRow, ScoreRank, ScoreRow, SyncStatus } from '../types';

export function computeScoreRankOptions(scoreData: ScoreRow[]): ScoreRank[] {
  const values = Array.from(
    new Set(scoreData.map((row) => row.rank).filter((rank): rank is ScoreRank => rank !== null)),
  );
  return sortByOrder(values, SCORE_RANK_ORDER_MAP);
}

export function computeFcOptions(scoreData: ScoreRow[]): FcStatus[] {
  const values = Array.from(
    new Set(scoreData.map((row) => row.fc).filter((status): status is FcStatus => status !== null)),
  );
  return sortByOrder(values, FC_ORDER_MAP);
}

export function computeSyncOptions(scoreData: ScoreRow[]): SyncStatus[] {
  const values = Array.from(
    new Set(scoreData.map((row) => row.sync).filter((status): status is SyncStatus => status !== null)),
  );
  return sortByOrder(values, SYNC_ORDER_MAP);
}

interface BuildFilteredScoreRowsParams {
  scoreData: ScoreRow[];
  query: string;
  chartFilter: ScoreRow['chartType'][];
  difficultyFilter: ScoreRow['difficulty'][];
  versionSelection: string;
  versionOptions: string[];
  rankFilter: ScoreRank[];
  fcFilter: FcStatus[];
  syncFilter: SyncStatus[];
  achievementMin: number;
  achievementMax: number;
  internalMin: number;
  internalMax: number;
  daysMin: number;
  daysMax: number;
  scoreSortKey: ScoreSortKey;
  scoreSortDesc: boolean;
}

export function buildFilteredScoreRows({
  scoreData,
  query,
  chartFilter,
  difficultyFilter,
  versionSelection,
  versionOptions,
  rankFilter,
  fcFilter,
  syncFilter,
  achievementMin,
  achievementMax,
  internalMin,
  internalMax,
  daysMin,
  daysMax,
  scoreSortKey,
  scoreSortDesc,
}: BuildFilteredScoreRowsParams): ScoreRow[] {
  const latestVersions = versionOptions.slice(-2);
  const latestSet = new Set(latestVersions);
  const oldSet = new Set(versionOptions.filter((version) => !latestSet.has(version)));

  const rows = scoreData.filter((row) => {
    const targetText = `${row.title} ${row.version ?? ''} ${row.level ?? ''}`;
    if (!includesText(targetText, query)) {
      return false;
    }

    if (!chartFilter.includes(row.chartType)) {
      return false;
    }

    if (!difficultyFilter.includes(row.difficulty)) {
      return false;
    }

    if (versionSelection === 'NEW') {
      if (!row.version || !latestSet.has(row.version)) {
        return false;
      }
    } else if (versionSelection === 'OLD') {
      if (!row.version || !oldSet.has(row.version)) {
        return false;
      }
    } else if (versionSelection !== 'ALL') {
      if (!row.version || row.version !== versionSelection) {
        return false;
      }
    }

    if (rankFilter.length > 0 && (!row.rank || !rankFilter.includes(row.rank))) {
      return false;
    }

    if (fcFilter.length > 0 && (!row.fc || !fcFilter.includes(row.fc))) {
      return false;
    }

    if (syncFilter.length > 0 && (!row.sync || !syncFilter.includes(row.sync))) {
      return false;
    }

    const achievementPercent = row.achievementPercent ?? 0;
    if (achievementPercent < achievementMin || achievementPercent > achievementMax) {
      return false;
    }

    if (row.internalLevel === null || row.internalLevel < internalMin || row.internalLevel > internalMax) {
      return false;
    }

    if (
      row.daysSinceLastPlayed !== null &&
      (row.daysSinceLastPlayed < daysMin || row.daysSinceLastPlayed > daysMax)
    ) {
      return false;
    }

    return true;
  });

  rows.sort((left, right) => {
    let result = 0;
    switch (scoreSortKey) {
      case 'title':
        result = left.title.localeCompare(right.title, 'ko');
        break;
      case 'achievement':
        result = compareNullableNumber(left.achievementPercent, right.achievementPercent);
        break;
      case 'rating':
        result = compareNullableNumber(left.ratingPoints, right.ratingPoints);
        break;
      case 'internal':
        result = compareNullableNumber(left.internalLevel, right.internalLevel);
        break;
      case 'dxRatio':
        result = compareNullableNumber(left.dxRatio, right.dxRatio);
        break;
      case 'playCount':
        result = compareNullableNumber(left.playCount, right.playCount);
        break;
      case 'lastPlayed':
        result = compareNullableNumber(left.latestPlayedAtUnix, right.latestPlayedAtUnix);
        break;
    }

    return scoreSortDesc ? -result : result;
  });

  return rows;
}

interface BuildFilteredPlaylogRowsParams {
  playlogData: PlaylogRow[];
  playlogQuery: string;
  playlogChartFilter: PlaylogRow['chartType'][];
  playlogDifficultyFilter: Array<NonNullable<PlaylogRow['difficulty']>>;
  playlogAchievementMin: number;
  playlogAchievementMax: number;
  playlogSortKey: PlaylogSortKey;
  playlogSortDesc: boolean;
  playlogDayStartUnix: number | null;
  playlogDayEndUnix: number | null;
}

export function buildFilteredPlaylogRows({
  playlogData,
  playlogQuery,
  playlogChartFilter,
  playlogDifficultyFilter,
  playlogAchievementMin,
  playlogAchievementMax,
  playlogSortKey,
  playlogSortDesc,
  playlogDayStartUnix,
  playlogDayEndUnix,
}: BuildFilteredPlaylogRowsParams): PlaylogRow[] {
  const rows = playlogData.filter((row) => {
    if (
      playlogDayStartUnix !== null &&
      playlogDayEndUnix !== null &&
      (row.playedAtUnix < playlogDayStartUnix || row.playedAtUnix >= playlogDayEndUnix)
    ) {
      return false;
    }

    if (!includesText(`${row.title} ${row.playedAtLabel ?? ''}`, playlogQuery)) {
      return false;
    }

    if (!playlogChartFilter.includes(row.chartType)) {
      return false;
    }

    if (row.difficulty !== null && !playlogDifficultyFilter.includes(row.difficulty)) {
      return false;
    }

    if (
      row.achievementPercent !== null &&
      (row.achievementPercent < playlogAchievementMin || row.achievementPercent > playlogAchievementMax)
    ) {
      return false;
    }

    return true;
  });

  rows.sort((left, right) => {
    let result = 0;
    switch (playlogSortKey) {
      case 'playedAt':
        result = left.playedAtUnix - right.playedAtUnix;
        break;
      case 'achievement':
        result = compareNullableNumber(left.achievementPercent, right.achievementPercent);
        break;
      case 'rating':
        result = compareNullableNumber(left.ratingPoints, right.ratingPoints);
        break;
      case 'dxRatio':
        result = compareNullableNumber(left.dxRatio, right.dxRatio);
        break;
      case 'playCount':
        result = compareNullableNumber(left.creditId, right.creditId);
        break;
      case 'title':
        result = left.title.localeCompare(right.title, 'ko');
        break;
    }

    return playlogSortDesc ? -result : result;
  });

  return rows;
}
