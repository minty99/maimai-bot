import type { FcStatus, SyncStatus } from '../types';

export interface NumericRangePreset<T extends string = string> {
  id: T;
  label: string;
  min: number;
  max: number;
}

export interface DirectionalRangeSelectionState<T extends string> {
  anchor: T;
  last: T;
  direction: -1 | 0 | 1;
}

export const ALL_FILTER_PRESET_ID = 'ALL';
export const NA_FILTER_OPTION_ID = 'N/A';

export const DEFAULT_SCORE_FILTERS = {
  versionSelection: 'ALL',
  achievementMin: 0,
  achievementMax: 101,
  internalMin: 1,
  internalMax: 15.5,
  daysMin: 0,
  daysMax: 2000,
} as const;

export type InternalLevelPresetId =
  | typeof ALL_FILTER_PRESET_ID
  | '~10+'
  | '11'
  | '11+'
  | '12'
  | '12+'
  | '13'
  | '13+'
  | '14'
  | '14+'
  | '15';

export const INTERNAL_LEVEL_CONTIGUOUS_PRESET_ORDER: Exclude<InternalLevelPresetId, 'ALL'>[] = [
  '~10+',
  '11',
  '11+',
  '12',
  '12+',
  '13',
  '13+',
  '14',
  '14+',
  '15',
];

export const INTERNAL_LEVEL_PRESETS: NumericRangePreset<InternalLevelPresetId>[] = [
  {
    id: ALL_FILTER_PRESET_ID,
    label: 'ALL',
    min: DEFAULT_SCORE_FILTERS.internalMin,
    max: DEFAULT_SCORE_FILTERS.internalMax,
  },
  { id: '~10+', label: '~10+', min: 1, max: 10.9 },
  { id: '11', label: '11', min: 11, max: 11.5 },
  { id: '11+', label: '11+', min: 11.6, max: 11.9 },
  { id: '12', label: '12', min: 12, max: 12.5 },
  { id: '12+', label: '12+', min: 12.6, max: 12.9 },
  { id: '13', label: '13', min: 13, max: 13.5 },
  { id: '13+', label: '13+', min: 13.6, max: 13.9 },
  { id: '14', label: '14', min: 14, max: 14.5 },
  { id: '14+', label: '14+', min: 14.6, max: 14.9 },
  { id: '15', label: '15', min: 15, max: 15.5 },
];

export type ScoreAchievementPresetId =
  | typeof ALL_FILTER_PRESET_ID
  | 'SSS+'
  | 'SSS'
  | 'SS+'
  | 'SS'
  | 'S+'
  | 'S'
  | '~AAA';

export const SCORE_ACHIEVEMENT_PRESET_ORDER: Exclude<ScoreAchievementPresetId, 'ALL'>[] = [
  'SSS+',
  'SSS',
  'SS+',
  'SS',
  'S+',
  'S',
  '~AAA',
];

export const SCORE_ACHIEVEMENT_PRESETS: NumericRangePreset<ScoreAchievementPresetId>[] = [
  {
    id: ALL_FILTER_PRESET_ID,
    label: 'ALL',
    min: DEFAULT_SCORE_FILTERS.achievementMin,
    max: DEFAULT_SCORE_FILTERS.achievementMax,
  },
  { id: 'SSS+', label: 'SSS+', min: 100.5, max: 101 },
  { id: 'SSS', label: 'SSS', min: 100, max: 100.4999 },
  { id: 'SS+', label: 'SS+', min: 99.5, max: 99.9999 },
  { id: 'SS', label: 'SS', min: 99, max: 99.4999 },
  { id: 'S+', label: 'S+', min: 98, max: 98.9999 },
  { id: 'S', label: 'S', min: 97, max: 97.9999 },
  { id: '~AAA', label: '~AAA', min: 0, max: 96.9999 },
];

export type FcFilterOptionId = typeof ALL_FILTER_PRESET_ID | FcStatus | typeof NA_FILTER_OPTION_ID;
export const FC_FILTER_PRESET_ORDER: Exclude<FcFilterOptionId, 'ALL'>[] = [
  'AP+',
  'AP',
  'FC+',
  'FC',
  'N/A',
];
export const FC_FILTER_OPTIONS: FcFilterOptionId[] = [ALL_FILTER_PRESET_ID, ...FC_FILTER_PRESET_ORDER];

export type SyncFilterOptionId = typeof ALL_FILTER_PRESET_ID | SyncStatus | typeof NA_FILTER_OPTION_ID;
export const SYNC_FILTER_PRESET_ORDER: Exclude<SyncFilterOptionId, 'ALL'>[] = [
  'FDX+',
  'FDX',
  'FS+',
  'FS',
  'SYNC',
  'N/A',
];
export const SYNC_FILTER_OPTIONS: SyncFilterOptionId[] = [ALL_FILTER_PRESET_ID, ...SYNC_FILTER_PRESET_ORDER];

const RANGE_EPSILON = 0.00005;

function areNumbersClose(left: number, right: number): boolean {
  return Math.abs(left - right) <= RANGE_EPSILON;
}

export function getPresetSelectionRange<T extends string>(
  presets: readonly NumericRangePreset<T>[],
  selection: readonly T[],
): { min: number; max: number } | null {
  if (selection.length === 0) {
    return null;
  }

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;

  for (const id of selection) {
    const preset = presets.find((item) => item.id === id);
    if (!preset) {
      return null;
    }
    min = Math.min(min, preset.min);
    max = Math.max(max, preset.max);
  }

  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return null;
  }

  return { min, max };
}

export function resolvePresetSelectionFromRange<T extends string>(
  presets: readonly NumericRangePreset<T>[],
  min: number,
  max: number,
): T[] {
  for (let startIndex = 0; startIndex < presets.length; startIndex += 1) {
    let sliceMin = Number.POSITIVE_INFINITY;
    let sliceMax = Number.NEGATIVE_INFINITY;

    for (let endIndex = startIndex; endIndex < presets.length; endIndex += 1) {
      const preset = presets[endIndex];
      sliceMin = Math.min(sliceMin, preset.min);
      sliceMax = Math.max(sliceMax, preset.max);

      if (areNumbersClose(sliceMin, min) && areNumbersClose(sliceMax, max)) {
        return presets.slice(startIndex, endIndex + 1).map((item) => item.id);
      }
    }
  }

  return [];
}

function selectionSliceFromOrder<T extends string>(order: readonly T[], left: T, right: T): T[] {
  const leftIndex = order.indexOf(left);
  const rightIndex = order.indexOf(right);
  if (leftIndex === -1 || rightIndex === -1) {
    return [right];
  }

  return order.slice(Math.min(leftIndex, rightIndex), Math.max(leftIndex, rightIndex) + 1);
}

export function updateDirectionalRangeSelection<T extends string>(params: {
  order: readonly T[];
  currentSelection: readonly T[];
  currentState: DirectionalRangeSelectionState<T> | null;
  clicked: T | typeof ALL_FILTER_PRESET_ID;
}): { selection: Array<T | typeof ALL_FILTER_PRESET_ID>; state: DirectionalRangeSelectionState<T> | null } {
  const {
    order,
    currentSelection,
    currentState,
    clicked,
  } = params;

  if (clicked === ALL_FILTER_PRESET_ID) {
    return { selection: [ALL_FILTER_PRESET_ID], state: null };
  }

  const normalizedSelection = currentSelection.filter((item): item is T => item !== ALL_FILTER_PRESET_ID);
  if (normalizedSelection.length === 1 && normalizedSelection[0] === clicked) {
    return { selection: [ALL_FILTER_PRESET_ID], state: null };
  }

  if (normalizedSelection.length === 0) {
    return {
      selection: [clicked],
      state: { anchor: clicked, last: clicked, direction: 0 },
    };
  }

  if (currentState === null) {
    if (normalizedSelection.length !== 1) {
      return {
        selection: [clicked],
        state: { anchor: clicked, last: clicked, direction: 0 },
      };
    }

    return {
      selection: selectionSliceFromOrder(order, normalizedSelection[0], clicked),
      state: {
        anchor: normalizedSelection[0],
        last: clicked,
        direction: Math.sign(order.indexOf(clicked) - order.indexOf(normalizedSelection[0])) as -1 | 0 | 1,
      },
    };
  }

  const lastIndex = order.indexOf(currentState.last);
  const clickedIndex = order.indexOf(clicked);
  if (lastIndex === -1 || clickedIndex === -1) {
    return {
      selection: [clicked],
      state: { anchor: clicked, last: clicked, direction: 0 },
    };
  }

  const step = Math.sign(clickedIndex - lastIndex) as -1 | 0 | 1;
  if (step === 0) {
    return {
      selection: [clicked],
      state: { anchor: clicked, last: clicked, direction: 0 },
    };
  }

  if (currentState.direction === 0 || currentState.direction === step) {
    return {
      selection: selectionSliceFromOrder(order, currentState.anchor, clicked),
      state: {
        anchor: currentState.anchor,
        last: clicked,
        direction: currentState.direction === 0 ? step : currentState.direction,
      },
    };
  }

  return {
    selection: [clicked],
    state: { anchor: clicked, last: clicked, direction: 0 },
  };
}
