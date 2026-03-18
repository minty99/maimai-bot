import {
  FC_ORDER_MAP,
  SCORE_RANK_ORDER_MAP,
  ScoreSortKey,
  SYNC_ORDER_MAP,
  VERSION_ORDER_MAP,
  PlaylogSortKey,
} from "./constants";
import {
  aliasValues,
  compareNullableNumber,
  includesText,
  sortByOrder,
} from "./utils";

import {
  ALL_FILTER_PRESET_ID,
  NA_FILTER_OPTION_ID,
} from "./scoreFilterPresets";
import type { PlaylogRow, ScoreRank, ScoreRow } from "../types";

export function computeScoreRankOptions(scoreData: ScoreRow[], locale: string): ScoreRank[] {
  const values = Array.from(
    new Set(
      scoreData
        .map((row) => row.rank)
        .filter((rank): rank is ScoreRank => rank !== null),
    ),
  );
  return sortByOrder(values, SCORE_RANK_ORDER_MAP, locale);
}

interface BuildFilteredScoreRowsParams {
  scoreData: ScoreRow[];
  locale: string;
  query: string;
  chartFilter: ScoreRow["chartType"][];
  difficultyFilter: ScoreRow["difficulty"][];
  versionSelection: string;
  versionOptions: string[];
  fcFilter: string[];
  syncFilter: string[];
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
  locale,
  query,
  chartFilter,
  difficultyFilter,
  versionSelection,
  versionOptions,
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
  const oldSet = new Set(
    versionOptions.filter((version) => !latestSet.has(version)),
  );

  const isIncluded = (row: ScoreRow): boolean => {
    const targetText = `${row.title} ${aliasValues(row.aliases, "en").join(" ")} ${aliasValues(row.aliases, "ko").join(" ")} ${row.version ?? ""} ${row.level ?? ""}`;
    if (!includesText(targetText, query)) {
      return false;
    }

    if (!chartFilter.includes(row.chartType)) {
      return false;
    }

    if (!difficultyFilter.includes(row.difficulty)) {
      return false;
    }

    if (versionSelection === "NEW") {
      if (!row.version || !latestSet.has(row.version)) {
        return false;
      }
    } else if (versionSelection === "OLD") {
      if (!row.version || !oldSet.has(row.version)) {
        return false;
      }
    } else if (versionSelection !== "ALL") {
      if (!row.version || row.version !== versionSelection) {
        return false;
      }
    }

    if (fcFilter.length > 0 && !fcFilter.includes(ALL_FILTER_PRESET_ID)) {
      const includeNull = fcFilter.includes(NA_FILTER_OPTION_ID);
      const selectedStatuses = new Set(
        fcFilter.filter((value) => value !== NA_FILTER_OPTION_ID),
      );
      if (row.fc === null) {
        if (!includeNull) {
          return false;
        }
      } else if (!selectedStatuses.has(row.fc)) {
        return false;
      }
    }

    if (syncFilter.length > 0 && !syncFilter.includes(ALL_FILTER_PRESET_ID)) {
      const includeNull = syncFilter.includes(NA_FILTER_OPTION_ID);
      const selectedStatuses = new Set(
        syncFilter.filter((value) => value !== NA_FILTER_OPTION_ID),
      );
      if (row.sync === null) {
        if (!includeNull) {
          return false;
        }
      } else if (!selectedStatuses.has(row.sync)) {
        return false;
      }
    }

    const achievementPercent = row.achievementPercent ?? 0;
    if (
      achievementPercent < achievementMin ||
      achievementPercent > achievementMax
    ) {
      return false;
    }

    if (
      row.internalLevel === null ||
      row.internalLevel < internalMin ||
      row.internalLevel > internalMax
    ) {
      return false;
    }

    if (
      row.daysSinceLastPlayed !== null &&
      (row.daysSinceLastPlayed < daysMin || row.daysSinceLastPlayed > daysMax)
    ) {
      return false;
    }

    return true;
  };

  const rows = scoreData.filter(isIncluded);

  rows.sort((left, right) => {
    let result = 0;
    switch (scoreSortKey) {
      case "title":
        result = left.title.localeCompare(right.title, locale);
        break;
      case "achievement":
        result = compareNullableNumber(
          left.achievementPercent,
          right.achievementPercent,
        );
        break;
      case "rating":
        result = compareNullableNumber(left.rating, right.rating);
        break;
      case "internal":
        result = compareNullableNumber(left.internalLevel, right.internalLevel);
        break;
      case "dxRatio":
        result = compareNullableNumber(left.dxRatio, right.dxRatio);
        break;
      case "playCount":
        result = compareNullableNumber(left.playCount, right.playCount);
        break;
      case "lastPlayed":
        result = compareNullableNumber(
          left.latestPlayedAtUnix,
          right.latestPlayedAtUnix,
        );
        break;
      case "fc": {
        const leftFc = left.fc !== null ? (FC_ORDER_MAP.get(left.fc) ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
        const rightFc = right.fc !== null ? (FC_ORDER_MAP.get(right.fc) ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
        result = rightFc - leftFc;
        break;
      }
      case "sync": {
        const leftSync = left.sync !== null ? (SYNC_ORDER_MAP.get(left.sync) ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
        const rightSync = right.sync !== null ? (SYNC_ORDER_MAP.get(right.sync) ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
        result = rightSync - leftSync;
        break;
      }
      case "version": {
        const leftVer = left.version !== null ? (VERSION_ORDER_MAP.get(left.version) ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
        const rightVer = right.version !== null ? (VERSION_ORDER_MAP.get(right.version) ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
        result = leftVer - rightVer;
        break;
      }
    }

    return scoreSortDesc ? -result : result;
  });

  return rows;
}

interface BuildFilteredPlaylogRowsParams {
  playlogData: PlaylogRow[];
  locale: string;
  playlogQuery: string;
  playlogChartFilter: PlaylogRow["chartType"][];
  playlogDifficultyFilter: Array<NonNullable<PlaylogRow["difficulty"]>>;
  playlogAchievementMin: number;
  playlogAchievementMax: number;
  playlogBestOnly: boolean;
  playlogNewRecordOnly: boolean;
  playlogSortKey: PlaylogSortKey;
  playlogSortDesc: boolean;
  playlogDayStartUnix: number | null;
  playlogDayEndUnix: number | null;
}

function playlogChartIdentity(row: PlaylogRow): string {
  return `${row.songKey}::${row.chartType}::${row.difficulty ?? 'null'}`;
}

function isBetterPlaylogCandidate(left: PlaylogRow, right: PlaylogRow): boolean {
  const leftAchievement = left.achievementX10000 ?? Number.NEGATIVE_INFINITY;
  const rightAchievement = right.achievementX10000 ?? Number.NEGATIVE_INFINITY;
  if (leftAchievement !== rightAchievement) {
    return leftAchievement > rightAchievement;
  }

  if (left.isNewRecord !== right.isNewRecord) {
    return left.isNewRecord;
  }

  if (left.playedAtUnix !== right.playedAtUnix) {
    return left.playedAtUnix > right.playedAtUnix;
  }

  return left.key > right.key;
}

function pickBestPlaylogRows(rows: PlaylogRow[]): PlaylogRow[] {
  const bestRows = new Map<string, PlaylogRow>();

  for (const row of rows) {
    const identity = playlogChartIdentity(row);
    const current = bestRows.get(identity);
    if (!current || isBetterPlaylogCandidate(row, current)) {
      bestRows.set(identity, row);
    }
  }

  return Array.from(bestRows.values());
}

export function buildFilteredPlaylogRows({
  playlogData,
  locale,
  playlogQuery,
  playlogChartFilter,
  playlogDifficultyFilter,
  playlogAchievementMin,
  playlogAchievementMax,
  playlogBestOnly,
  playlogNewRecordOnly,
  playlogSortKey,
  playlogSortDesc,
  playlogDayStartUnix,
  playlogDayEndUnix,
}: BuildFilteredPlaylogRowsParams): PlaylogRow[] {
  let rows = playlogData.filter((row) => {
    if (
      playlogDayStartUnix !== null &&
      playlogDayEndUnix !== null &&
      (row.playedAtUnix < playlogDayStartUnix ||
        row.playedAtUnix >= playlogDayEndUnix)
    ) {
      return false;
    }

    if (
      !includesText(
        `${row.title} ${aliasValues(row.aliases, "en").join(" ")} ${aliasValues(row.aliases, "ko").join(" ")} ${row.playedAtLabel ?? ""}`,
        playlogQuery,
      )
    ) {
      return false;
    }

    if (!playlogChartFilter.includes(row.chartType)) {
      return false;
    }

    if (
      row.difficulty !== null &&
      !playlogDifficultyFilter.includes(row.difficulty)
    ) {
      return false;
    }

    if (
      row.achievementPercent !== null &&
      (row.achievementPercent < playlogAchievementMin ||
        row.achievementPercent > playlogAchievementMax)
    ) {
      return false;
    }

    return true;
  });

  if (playlogBestOnly) {
    rows = pickBestPlaylogRows(rows);
  }

  if (playlogNewRecordOnly) {
    rows = rows.filter((row) => row.isNewRecord);
  }

  rows.sort((left, right) => {
    let result = 0;
    switch (playlogSortKey) {
      case "playedAt":
        result = left.playedAtUnix - right.playedAtUnix;
        break;
      case "achievement":
        result = compareNullableNumber(
          left.achievementPercent,
          right.achievementPercent,
        );
        break;
      case "rating":
        result = compareNullableNumber(left.rating, right.rating);
        break;
      case "dxRatio":
        result = compareNullableNumber(left.dxRatio, right.dxRatio);
        break;
      case "playCount":
        result = compareNullableNumber(left.creditId, right.creditId);
        break;
      case "title":
        result = left.title.localeCompare(right.title, locale);
        break;
    }

    return playlogSortDesc ? -result : result;
  });

  return rows;
}
