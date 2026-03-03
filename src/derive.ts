import type {
  ChartType,
  DifficultyCategory,
  PlayRecordApiResponse,
  PlaylogRow,
  ScoreApiResponse,
  ScoreRow,
  SongInfoResponse,
} from './types';
import { normalizeTitleKey } from './api';

export function chartKey(
  title: string,
  chartType: ChartType,
  difficulty: DifficultyCategory,
): string {
  return `${normalizeTitleKey(title)}::${chartType}::${difficulty}`;
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
  songInfoByTitle: Map<string, SongInfoResponse>,
): ScoreRow[] {
  return scores.map((score) => {
    const normalizedTitle = normalizeTitleKey(score.title);
    const key = chartKey(score.title, score.chart_type, score.diff_category);

    const songInfo = songInfoByTitle.get(normalizedTitle);
    const sheet = findMatchingSheet(songInfo, score.chart_type, score.diff_category);

    const latestPlayedAtUnix = parseMaimaiPlayedAtToUnix(score.last_played_at);

    return {
      key,
      title: score.title,
      normalizedTitle,
      chartType: score.chart_type,
      difficulty: score.diff_category,
      achievementX10000: score.achievement_x10000,
      achievementPercent: toAchievementPercent(score.achievement_x10000),
      rank: score.rank,
      fc: score.fc,
      sync: score.sync,
      dxScore: score.dx_score,
      dxScoreMax: score.dx_score_max,
      dxRatio: toDxRatio(score.dx_score, score.dx_score_max),
      ratingPoints: score.rating_points ?? toRatingPoints(
        sheet?.internal_level ?? null,
        score.achievement_x10000,
        score.fc,
      ),
      level: sheet?.level ?? null,
      internalLevel: sheet?.internal_level ?? null,
      userLevel: sheet?.user_level ?? null,
      version: sheet?.version ?? null,
      imageName: songInfo?.image_name ?? null,
      latestPlayedAtUnix,
      latestPlayedAtLabel: score.last_played_at ?? toDateLabel(latestPlayedAtUnix),
      daysSinceLastPlayed: daysSince(latestPlayedAtUnix),
      playCount: score.play_count ?? null,
    };
  });
}

export function buildPlaylogRows(
  playlogs: PlayRecordApiResponse[],
  songInfoByTitle: Map<string, SongInfoResponse>,
): PlaylogRow[] {
  return playlogs.map((log, index) => {
    const normalizedTitle = normalizeTitleKey(log.title);
    const songInfo = songInfoByTitle.get(normalizedTitle);
    const sheet =
      log.diff_category === null
        ? null
        : findMatchingSheet(songInfo, log.chart_type, log.diff_category);

    return {
      key: `${log.played_at_unixtime}-${index}`,
      title: log.title,
      normalizedTitle,
      chartType: log.chart_type,
      difficulty: log.diff_category,
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
        sheet?.internal_level ?? null,
        log.achievement_x10000,
        log.fc,
      ),
      creditPlayCount: log.credit_play_count,
      isNewRecord: (log.achievement_new_record ?? 0) > 0,
      isFirstPlay: (log.first_play ?? 0) > 0,
      imageName: songInfo?.image_name ?? null,
    };
  });
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
