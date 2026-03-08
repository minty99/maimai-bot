import type {
  ChartType,
  DifficultyCategory,
  PlayRecordApiResponse,
  ScoreHistoryPoint,
  PlaylogRow,
  ScoreApiResponse,
  ScoreRow,
  SongDetailRow,
  SongInfoResponse,
  SongSheetResponse,
} from './types';
import { chartIdentityKey, songIdentityKey } from './songIdentity';
import { CHART_TYPES, DIFFICULTIES } from './app/constants';

export function chartKey(
  title: string,
  genre: string,
  artist: string,
  chartType: ChartType,
  difficulty: DifficultyCategory,
): string {
  return chartIdentityKey(title, genre, artist, chartType, difficulty);
}

function findMatchingSheet(
  songInfo: SongInfoResponse | undefined,
  chartType: ChartType,
  difficulty: DifficultyCategory,
) {
  if (!songInfo) {
    return null;
  }

  const matches = songInfo.sheets.filter(
    (item) => item.chart_type === chartType && item.difficulty === difficulty,
  );

  return matches.find((item) => item.region.intl) ?? matches[0] ?? null;
}

function listPreferredSheets(songInfo: SongInfoResponse): SongSheetResponse[] {
  const preferredSheets = new Map<string, SongSheetResponse>();

  for (const sheet of songInfo.sheets) {
    const key = `${sheet.chart_type}::${sheet.difficulty}`;
    const existing = preferredSheets.get(key);
    if (!existing || (!existing.region.intl && sheet.region.intl)) {
      preferredSheets.set(key, sheet);
    }
  }

  return Array.from(preferredSheets.values());
}

function toUnixMillis(unixtime: number): number {
  if (unixtime > 10_000_000_000) {
    return unixtime;
  }
  return unixtime * 1000;
}

export function toDateLabel(unixtime: number | null): string | null {
  if (unixtime === null) {
    return null;
  }

  const ms = toUnixMillis(unixtime);
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) {
    return null;
  }

  return date.toLocaleString('ko-KR', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function toAchievementPercent(achievementX10000: number | null): number | null {
  if (achievementX10000 === null) {
    return null;
  }
  return achievementX10000 / 10000;
}

export function toDxRatio(dxScore: number | null, dxScoreMax: number | null): number | null {
  if (dxScore === null || dxScoreMax === null || dxScoreMax <= 0) {
    return null;
  }
  return dxScore / dxScoreMax;
}

function estimateInternalLevel(level: string | null | undefined): number | null {
  const normalized = level?.trim();
  if (!normalized) {
    return null;
  }

  const match = /^(\d+)(\+)?$/.exec(normalized);
  if (!match) {
    return null;
  }

  const [, baseText, plus] = match;
  const base = Number(baseText);
  if (!Number.isFinite(base)) {
    return null;
  }

  return base + (plus ? 0.5 : 0);
}

function coefficientForAchievement(achievementPercent: number): number {
  const achievement = Math.min(achievementPercent, 100.5);

  if (achievement >= 100.5) return 22.4;
  if (achievement >= 100.4999) return 22.2;
  if (achievement >= 100.0) return 21.6;
  if (achievement >= 99.9999) return 21.4;
  if (achievement >= 99.5) return 21.1;
  if (achievement >= 99.0) return 20.8;
  if (achievement >= 98.9999) return 20.6;
  if (achievement >= 98.0) return 20.3;
  if (achievement >= 97.0) return 20.0;
  if (achievement >= 96.9999) return 17.6;
  if (achievement >= 94.0) return 16.8;
  if (achievement >= 90.0) return 15.2;
  if (achievement >= 80.0) return 13.6;
  if (achievement >= 79.9999) return 12.8;
  if (achievement >= 75.0) return 12.0;
  if (achievement >= 70.0) return 11.2;
  if (achievement >= 60.0) return 9.6;
  if (achievement >= 50.0) return 8.0;
  if (achievement >= 40.0) return 6.4;
  if (achievement >= 30.0) return 4.8;
  if (achievement >= 20.0) return 3.2;
  if (achievement >= 10.0) return 1.6;
  return 0.0;
}

function isApLike(fc: string | null): boolean {
  return fc === 'AP' || fc === 'AP+';
}

export function toRatingPoints(
  internalLevel: number | null,
  achievementX10000: number | null,
  fc: string | null,
): number | null {
  if (internalLevel === null || achievementX10000 === null) {
    return null;
  }

  const achievementPercent = achievementX10000 / 10000;
  const achievement = Math.min(achievementPercent, 100.5);
  const coefficient = coefficientForAchievement(achievementPercent);
  const base = Math.floor((coefficient * internalLevel * achievement) / 100);
  const points = Number.isFinite(base) && base > 0 ? base : 0;
  return isApLike(fc) ? points + 1 : points;
}

export function daysSince(unixtime: number | null): number | null {
  if (unixtime === null) {
    return null;
  }
  const ms = toUnixMillis(unixtime);
  const diffMs = Date.now() - ms;
  if (diffMs < 0) {
    return 0;
  }
  return Math.floor(diffMs / (1000 * 60 * 60 * 24));
}

function parseMaimaiPlayedAtToUnix(playedAt: string | null | undefined): number | null {
  const text = playedAt?.trim();
  if (!text) {
    return null;
  }

  const match = /^(\d{4})\/(\d{2})\/(\d{2})\s+(\d{2}):(\d{2})$/.exec(text);
  if (!match) {
    return null;
  }

  const [, yearText, monthText, dayText, hourText, minuteText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const date = new Date(year, month - 1, day, hour, minute, 0, 0);

  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return date.getTime();
}

export function buildScoreRows(
  scores: ScoreApiResponse[],
  songInfoByIdentity: Map<string, SongInfoResponse>,
): ScoreRow[] {
  const scoreByKey = new Map<string, ScoreApiResponse>();
  for (const score of scores) {
    scoreByKey.set(
      chartKey(
        score.title,
        score.genre,
        score.artist,
        score.chart_type,
        score.diff_category,
      ),
      score,
    );
  }

  const rows: ScoreRow[] = [];
  const includedKeys = new Set<string>();

  const buildRow = (
    title: string,
    genre: string,
    artist: string,
    chartType: ChartType,
    difficulty: DifficultyCategory,
    songInfo?: SongInfoResponse,
    sheet?: SongSheetResponse | null,
  ): ScoreRow => {
    const songKey = songIdentityKey(title, genre, artist);
    const key = chartKey(title, genre, artist, chartType, difficulty);
    const matchedSheet = sheet ?? findMatchingSheet(songInfo, chartType, difficulty);
    const score = scoreByKey.get(key) ?? null;
    const estimatedInternalLevel = estimateInternalLevel(matchedSheet?.level);
    const internalLevel = matchedSheet?.internal_level ?? estimatedInternalLevel;
    const isInternalLevelEstimated =
      matchedSheet?.internal_level === null && internalLevel !== null;
    const latestPlayedAtUnix = parseMaimaiPlayedAtToUnix(score?.last_played_at);

    return {
      key,
      songKey,
      title,
      genre,
      artist,
      chartType,
      difficulty,
      achievementX10000: score?.achievement_x10000 ?? null,
      achievementPercent: toAchievementPercent(score?.achievement_x10000 ?? null),
      rank: score?.rank ?? null,
      fc: score?.fc ?? null,
      sync: score?.sync ?? null,
      dxScore: score?.dx_score ?? null,
      dxScoreMax: score?.dx_score_max ?? null,
      dxRatio: toDxRatio(score?.dx_score ?? null, score?.dx_score_max ?? null),
      ratingPoints: score?.rating_points ?? toRatingPoints(
        internalLevel,
        score?.achievement_x10000 ?? null,
        score?.fc ?? null,
      ),
      level: matchedSheet?.level ?? null,
      internalLevel,
      isInternalLevelEstimated,
      version: matchedSheet?.version ?? null,
      imageName: songInfo?.image_name ?? null,
      latestPlayedAtUnix,
      latestPlayedAtLabel: score?.last_played_at ?? toDateLabel(latestPlayedAtUnix),
      daysSinceLastPlayed: daysSince(latestPlayedAtUnix),
      playCount: score?.play_count ?? null,
    };
  };

  for (const songInfo of songInfoByIdentity.values()) {
    for (const sheet of listPreferredSheets(songInfo)) {
      includedKeys.add(
        chartKey(
          songInfo.title,
          songInfo.genre,
          songInfo.artist,
          sheet.chart_type,
          sheet.difficulty,
        ),
      );
      rows.push(
        buildRow(
          songInfo.title,
          songInfo.genre,
          songInfo.artist,
          sheet.chart_type,
          sheet.difficulty,
          songInfo,
          sheet,
        ),
      );
    }
  }

  for (const score of scores) {
    const key = chartKey(
      score.title,
      score.genre,
      score.artist,
      score.chart_type,
      score.diff_category,
    );
    if (includedKeys.has(key)) {
      continue;
    }

    includedKeys.add(key);
    rows.push(
      buildRow(
        score.title,
        score.genre,
        score.artist,
        score.chart_type,
        score.diff_category,
      ),
    );
  }

  return rows;
}

export function buildPlaylogRows(
  playlogs: PlayRecordApiResponse[],
  songInfoByIdentity: Map<string, SongInfoResponse>,
): PlaylogRow[] {
  return playlogs.map((log, index) => {
    const genre = log.genre ?? '';
    const artist = log.artist ?? '';
    const songKey = songIdentityKey(log.title, genre, artist);
    const songInfo = songInfoByIdentity.get(songKey);
    const sheet =
      log.diff_category === null
        ? null
        : findMatchingSheet(songInfo, log.chart_type, log.diff_category);
    const estimatedInternalLevel = estimateInternalLevel(sheet?.level);
    const internalLevel = sheet?.internal_level ?? estimatedInternalLevel;
    const isInternalLevelEstimated = sheet?.internal_level === null && internalLevel !== null;

    return {
      key: `${log.played_at_unixtime}-${index}`,
      songKey,
      title: log.title,
      genre,
      artist,
      chartType: log.chart_type,
      difficulty: log.diff_category,
      level: sheet?.level ?? null,
      internalLevel,
      isInternalLevelEstimated,
      playedAtUnix: log.played_at_unixtime,
      playedAtLabel: log.played_at ?? toDateLabel(log.played_at_unixtime),
      track: log.track,
      achievementX10000: log.achievement_x10000,
      achievementPercent: toAchievementPercent(log.achievement_x10000),
      rank: log.score_rank,
      fc: log.fc,
      sync: log.sync,
      dxScore: log.dx_score,
      dxScoreMax: log.dx_score_max,
      dxRatio: toDxRatio(log.dx_score, log.dx_score_max),
      ratingPoints: log.rating_points ?? toRatingPoints(
        internalLevel,
        log.achievement_x10000,
        log.fc,
      ),
      creditId: log.credit_id,
      isNewRecord: (log.achievement_new_record ?? 0) > 0,
      imageName: songInfo?.image_name ?? null,
    };
  });
}

export function buildScoreHistoryPoints(
  playlogRows: PlaylogRow[],
  selectedRow: ScoreRow | null,
): ScoreHistoryPoint[] {
  if (!selectedRow) {
    return [];
  }

  const matchingRows = playlogRows
    .filter(
      (row) =>
        row.songKey === selectedRow.songKey &&
        row.chartType === selectedRow.chartType &&
        row.difficulty === selectedRow.difficulty &&
        row.achievementX10000 !== null &&
        row.achievementPercent !== null,
    )
    .sort((left, right) => {
      if (left.playedAtUnix !== right.playedAtUnix) {
        return left.playedAtUnix - right.playedAtUnix;
      }
      return left.key.localeCompare(right.key, 'ko');
    });

  const points: ScoreHistoryPoint[] = [];
  let bestAchievementX10000 = Number.NEGATIVE_INFINITY;

  for (const row of matchingRows) {
    if (row.achievementX10000 === null || row.achievementPercent === null) {
      continue;
    }
    if (row.achievementX10000 <= bestAchievementX10000) {
      continue;
    }

    bestAchievementX10000 = row.achievementX10000;
    points.push({
      key: row.key,
      playedAtUnix: row.playedAtUnix,
      playedAtLabel: row.playedAtLabel,
      achievementPercent: row.achievementPercent,
    });
  }

  return points;
}

const chartOrder = new Map(CHART_TYPES.map((value, index) => [value, index]));
const difficultyOrder = new Map(DIFFICULTIES.map((value, index) => [value, index]));

export function buildSongDetailRows(
  scoreRows: ScoreRow[],
  songKey: string | null,
): SongDetailRow[] {
  if (!songKey) {
    return [];
  }

  return scoreRows
    .filter((row) => row.songKey === songKey)
    .sort((left, right) => {
      const chartDelta =
        (chartOrder.get(left.chartType) ?? Number.MAX_SAFE_INTEGER) -
        (chartOrder.get(right.chartType) ?? Number.MAX_SAFE_INTEGER);
      if (chartDelta !== 0) {
        return chartDelta;
      }
      return (
        (difficultyOrder.get(left.difficulty) ?? Number.MAX_SAFE_INTEGER) -
        (difficultyOrder.get(right.difficulty) ?? Number.MAX_SAFE_INTEGER)
      );
    })
    .map((row) => ({
      key: row.key,
      songKey: row.songKey,
      title: row.title,
      genre: row.genre,
      artist: row.artist,
      imageName: row.imageName,
      chartType: row.chartType,
      difficulty: row.difficulty,
      level: row.level,
      internalLevel: row.internalLevel,
      isInternalLevelEstimated: row.isInternalLevelEstimated,
      achievementPercent: row.achievementPercent,
      rank: row.rank,
      fc: row.fc,
      sync: row.sync,
      dxScore: row.dxScore,
      dxScoreMax: row.dxScoreMax,
      lastPlayedAtLabel: row.latestPlayedAtLabel,
      playCount: row.playCount,
      version: row.version,
    }));
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
