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
          gap: 0.0,
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
    final gap = _roundToTenth(finalEnd - validStart);

    emit(LevelRangeState(start: validStart, end: finalEnd, gap: gap));
  }

  /// Increment level by 0.1, maintaining gap.
  void incrementLevel() {
    final newStart = _roundToTenth(state.start + AppConstants.defaultLevelStep);
    final newEnd = _roundToTenth(newStart + state.gap);

    final validStart = _clampLevel(newStart);
    final validEnd = _clampLevel(newEnd);
    final effectiveGap = _roundToTenth(validEnd - validStart);

    emit(state.copyWith(start: validStart, end: validEnd, gap: effectiveGap));
  }

  /// Decrement level by 0.1, maintaining gap.
  void decrementLevel() {
    final newStart = _roundToTenth(state.start - AppConstants.defaultLevelStep);
    final newEnd = _roundToTenth(newStart + state.gap);

    final validStart = _clampLevel(newStart);
    final validEnd = _clampLevel(newEnd);
    final effectiveGap = _roundToTenth(validEnd - validStart);

    emit(state.copyWith(start: validStart, end: validEnd, gap: effectiveGap));
  }

  /// Increment start level by gap, adjust end to maintain gap.
  @Deprecated('Use incrementLevel() instead')
  void incrementStart() {
    incrementLevel();
  }

  /// Decrement start level by gap, adjust end to maintain gap.
  @Deprecated('Use decrementLevel() instead')
  void decrementStart() {
    decrementLevel();
  }

  /// Adjust the gap between start and end.
  ///
  /// Keeps start fixed, adjusts end to maintain new gap.
  void adjustGap(double newGap) {
    _applyGap(newGap);
  }

  /// Increment gap by the default step size.
  void incrementGap() {
    _applyGap(state.gap + AppConstants.defaultLevelStep);
  }

  /// Decrement gap by the default step size.
  void decrementGap() {
    _applyGap(state.gap - AppConstants.defaultLevelStep);
  }

  /// Clamp a level value to the valid bounds.
  double _clampLevel(double level) {
    final clamped = level.clamp(
      AppConstants.minLevelBound,
      AppConstants.maxLevelBound,
    );
    return _roundToTenth(clamped.toDouble());
  }

  void _applyGap(double newGap) {
    final maxGap = _roundToTenth(AppConstants.maxLevelBound - state.start);
    final validGap = _roundToTenth(newGap.clamp(0.0, maxGap).toDouble());
    final newEnd = _roundToTenth(state.start + validGap);
    final validEnd = _clampLevel(newEnd);
    final effectiveGap = _roundToTenth(validEnd - state.start);

    emit(state.copyWith(gap: effectiveGap, end: validEnd));
  }

  double _roundToTenth(double value) {
    return (value * 10).roundToDouble() / 10.0;
  }
}
