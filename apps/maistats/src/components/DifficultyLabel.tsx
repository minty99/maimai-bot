import type { DifficultyCategory } from '../types';
import { formatDifficultyShort } from '../app/utils';

export function getDifficultyToneClass(value: DifficultyCategory | null | undefined): string {
  switch (value) {
    case 'BASIC':
      return 'diff-basic';
    case 'ADVANCED':
      return 'diff-advanced';
    case 'EXPERT':
      return 'diff-expert';
    case 'MASTER':
      return 'diff-master';
    case 'Re:MASTER':
      return 'diff-remaster';
    default:
      return 'diff-unknown';
  }
}

interface DifficultyLabelProps {
  difficulty: DifficultyCategory | null | undefined;
  short?: boolean;
  className?: string;
}

export function DifficultyLabel({
  difficulty,
  short = false,
  className,
}: DifficultyLabelProps) {
  if (!difficulty) {
    return <span className={className}>-</span>;
  }

  const label = short ? formatDifficultyShort(difficulty) : difficulty;
  const nextClassName = ['difficulty-label', getDifficultyToneClass(difficulty), className]
    .filter(Boolean)
    .join(' ');

  return <span className={nextClassName}>{label}</span>;
}
