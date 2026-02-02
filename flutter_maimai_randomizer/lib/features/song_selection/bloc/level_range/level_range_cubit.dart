import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../../core/constants/app_constants.dart';
import 'level_range_state.dart';

/// Cubit for managing level range state.
///
/// Handles level range updates with bounds checking and gap maintenance.
class LevelRangeCubit extends Cubit<LevelRangeState> {
  LevelRangeCubit()
    : super(
        const LevelRangeState(
          start: AppConstants.defaultMinLevel,
          end: AppConstants.defaultMaxLevel,
          gap: AppConstants.defaultLevelStep,
        ),
      );

  /// Update the entire range (start and end).
  ///
  /// Validates bounds and ensures end >= start.
  void updateRange(double start, double end) {
    final validStart = _clampLevel(start);
    final validEnd = _clampLevel(end);

    // Ensure end >= start
    final finalEnd = validEnd < validStart ? validStart : validEnd;
    final gap = finalEnd - validStart;

    emit(LevelRangeState(start: validStart, end: finalEnd, gap: gap));
  }

  /// Increment start level by gap, adjust end to maintain gap.
  void incrementStart() {
    final newStart = state.start + state.gap;
    final newEnd = newStart + state.gap;

    final validStart = _clampLevel(newStart);
    final validEnd = _clampLevel(newEnd);

    emit(state.copyWith(start: validStart, end: validEnd));
  }

  /// Decrement start level by gap, adjust end to maintain gap.
  void decrementStart() {
    final newStart = state.start - state.gap;
    final newEnd = newStart + state.gap;

    final validStart = _clampLevel(newStart);
    final validEnd = _clampLevel(newEnd);

    emit(state.copyWith(start: validStart, end: validEnd));
  }

  /// Adjust the gap between start and end.
  ///
  /// Keeps start fixed, adjusts end to maintain new gap.
  void adjustGap(double newGap) {
    final validGap = newGap.clamp(0.0, double.infinity);
    final newEnd = state.start + validGap;
    final validEnd = _clampLevel(newEnd);

    emit(state.copyWith(gap: validGap, end: validEnd));
  }

  /// Clamp a level value to the valid bounds.
  double _clampLevel(double level) {
    return level.clamp(AppConstants.minLevelBound, AppConstants.maxLevelBound);
  }
}
