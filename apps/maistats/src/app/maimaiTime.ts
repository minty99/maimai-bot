export function daysSince(unixtimeMs: number | null): number | null {
  if (unixtimeMs === null) {
    return null;
  }

  const diffMs = Date.now() - unixtimeMs;
  if (diffMs < 0) {
    return 0;
  }

  return Math.floor(diffMs / (1000 * 60 * 60 * 24));
}

export function parseMaimaiPlayedAtToUnix(playedAt: string | null | undefined): number | null {
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
