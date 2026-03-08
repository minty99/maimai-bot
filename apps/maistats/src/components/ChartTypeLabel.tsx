import type { ChartType } from '../types';

export function getChartTypeToneClass(value: ChartType | null | undefined): string {
  switch (value) {
    case 'STD':
      return 'chart-type-std';
    case 'DX':
      return 'chart-type-dx';
    default:
      return 'chart-type-unknown';
  }
}

interface ChartTypeLabelProps {
  chartType: ChartType | null | undefined;
  className?: string;
}

export function ChartTypeLabel({ chartType, className }: ChartTypeLabelProps) {
  if (!chartType) {
    return <span className={className}>-</span>;
  }

  const nextClassName = ['chart-type-badge', getChartTypeToneClass(chartType), className]
    .filter(Boolean)
    .join(' ');

  return <span className={nextClassName}>{chartType}</span>;
}
