import type { ReactNode } from 'react';

interface ToggleGroupProps<T extends string> {
  label: string;
  options: readonly T[];
  selected: T[];
  onToggle: (value: T) => void;
  formatLabel?: (value: T) => string;
  renderLabel?: (value: T) => ReactNode;
  optionClassName?: (value: T) => string | undefined;
}

export function ToggleGroup<T extends string>({
  label,
  options,
  selected,
  onToggle,
  formatLabel,
  renderLabel,
  optionClassName,
}: ToggleGroupProps<T>) {
  return (
    <div className="filter-block">
      <div className="filter-label">{label}</div>
      <div className="chip-row">
        {options.map((option) => (
          <button
            key={option}
            type="button"
            className={['chip', selected.includes(option) ? 'active' : '', optionClassName?.(option)]
              .filter(Boolean)
              .join(' ')}
            onClick={() => onToggle(option)}
          >
            {renderLabel ? renderLabel(option) : formatLabel ? formatLabel(option) : option}
          </button>
        ))}
      </div>
    </div>
  );
}
