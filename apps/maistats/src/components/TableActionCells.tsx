import { formatPercent } from '../app/utils';

export interface SongDetailTarget {
  title: string;
  genre: string;
  artist: string;
}

interface SongTitleButtonProps {
  target: SongDetailTarget | null;
  title: string;
  subtitle?: string | null;
  onOpenSongDetail: (target: SongDetailTarget) => void;
}

export function SongTitleButton({
  target,
  title,
  subtitle = null,
  onOpenSongDetail,
}: SongTitleButtonProps) {
  if (!target) {
    return (
      <>
        <span>{title}</span>
        {subtitle ? <small className="title-cell-subtitle">{subtitle}</small> : null}
      </>
    );
  }

  return (
    <>
      <button
        type="button"
        className="link-button"
        onClick={() => onOpenSongDetail(target)}
      >
        {title}
      </button>
      {subtitle ? <small className="title-cell-subtitle">{subtitle}</small> : null}
    </>
  );
}

interface AchievementHistoryButtonProps {
  achievementPercent: number | null;
  onOpenHistory?: (() => void) | null;
  isHighlighted?: boolean;
  variant?: 'score' | 'playlog';
}

export function AchievementHistoryButton({
  achievementPercent,
  onOpenHistory,
  isHighlighted = false,
  variant = 'score',
}: AchievementHistoryButtonProps) {
  if (achievementPercent === null) {
    return '-';
  }

  const value = variant === 'playlog'
    ? (
      <span className={`achievement-value ${isHighlighted ? 'achievement-value--new' : ''}`}>
        {formatPercent(achievementPercent)}
      </span>
    )
    : formatPercent(achievementPercent);

  if (!onOpenHistory) {
    return value;
  }

  const className = variant === 'score'
    ? 'achievement-history-button'
    : 'achievement-value-button';

  return (
    <button
      type="button"
      className={className}
      onClick={onOpenHistory}
    >
      {value}
    </button>
  );
}
