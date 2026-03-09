import type { DifficultyCategory } from '../types';
import { getDifficultyToneClass } from './DifficultyLabel';

interface LevelCellProps {
  internalLevel: number | null;
  isInternalLevelEstimated: boolean;
  difficulty: DifficultyCategory | null;
}

export function LevelCell({
  internalLevel,
  isInternalLevelEstimated,
  difficulty,
}: LevelCellProps) {
  if (internalLevel === null) {
    return '-';
  }

  const [whole, fraction = '0'] = internalLevel.toFixed(1).split('.');
  const toneClass = difficulty === null ? '' : getDifficultyToneClass(difficulty);
  const badgeClassName = toneClass ? `level-badge ${toneClass}` : 'level-badge';

  if (isInternalLevelEstimated) {
    return (
      <span className={badgeClassName}>
        <span className={`estimated-level ${toneClass}`.trim()}>
          {whole}
          <span className="estimated-level-fraction">.{fraction}</span>
        </span>
      </span>
    );
  }

  return <span className={badgeClassName}>{whole}.{fraction}</span>;
}
