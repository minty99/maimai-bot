import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../../core/constants/app_constants.dart';
import 'level_range_state.dart';

/// Cubit for managing level range state.
///
/// Handles level range updates with bounds checking, gap maintenance,
/// and SharedPreferences persistence.
class LevelRangeCubit extends Cubit<LevelRangeState> {
  LevelRangeCubit()
    : super(
        const LevelRangeState(
          start: AppConstants.defaultMinLevel,
          end: AppConstants.defaultMaxLevel,
          gap: 0.0,
        ),
      );

  static const String _levelStartKey = 'level_start';
  static const String _levelGapKey = 'level_gap';

  /// Initialize level range from SharedPreferences.
  ///
  /// Call this during app startup to load persisted settings.
  Future<void> initialize() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final savedStart = prefs.getDouble(_levelStartKey);
      final savedGap = prefs.getDouble(_levelGapKey);

      final start = savedStart ?? AppConstants.defaultMinLevel;
      final gap = savedGap ?? 0.0;
      final end = _roundToTenth(start + gap);

      final validStart = _clampLevel(start);
      final validEnd = _clampLevel(end);
      final effectiveGap = _roundToTenth(validEnd - validStart);

      emit(LevelRangeState(start: validStart, end: validEnd, gap: effectiveGap));
    } catch (e) {
      // If loading fails, keep default state
    }
  }

  /// Persist current level range to SharedPreferences.
  Future<void> _persist() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setDouble(_levelStartKey, state.start);
      await prefs.setDouble(_levelGapKey, state.gap);
    } catch (e) {
      // Handle persistence error silently
    }
  }

  /// Update the entire range (start and end).
  ///
  /// Validates bounds and ensures end >= start.
  Future<void> updateRange(double start, double end) async {
    final validStart = _clampLevel(start);
    final validEnd = _clampLevel(end);

    // Ensure end >= start
    final finalEnd = validEnd < validStart ? validStart : validEnd;
    final gap = _roundToTenth(finalEnd - validStart);

    emit(LevelRangeState(start: validStart, end: finalEnd, gap: gap));
    await _persist();
  }

  /// Increment level by 0.1, maintaining gap.
  Future<void> incrementLevel() async {
    final newStart = _roundToTenth(state.start + AppConstants.defaultLevelStep);
    final newEnd = _roundToTenth(newStart + state.gap);

    final validStart = _clampLevel(newStart);
    final validEnd = _clampLevel(newEnd);
    final effectiveGap = _roundToTenth(validEnd - validStart);

    emit(state.copyWith(start: validStart, end: validEnd, gap: effectiveGap));
    await _persist();
  }

  /// Decrement level by 0.1, maintaining gap.
  Future<void> decrementLevel() async {
    final newStart = _roundToTenth(state.start - AppConstants.defaultLevelStep);
    final newEnd = _roundToTenth(newStart + state.gap);

    final validStart = _clampLevel(newStart);
    final validEnd = _clampLevel(newEnd);
    final effectiveGap = _roundToTenth(validEnd - validStart);

    emit(state.copyWith(start: validStart, end: validEnd, gap: effectiveGap));
    await _persist();
  }

  /// Increment start level by gap, adjust end to maintain gap.
  @Deprecated('Use incrementLevel() instead')
  Future<void> incrementStart() async {
    await incrementLevel();
  }

  /// Decrement start level by gap, adjust end to maintain gap.
  @Deprecated('Use decrementLevel() instead')
  Future<void> decrementStart() async {
    await decrementLevel();
  }

  /// Adjust the gap between start and end.
  ///
  /// Keeps start fixed, adjusts end to maintain new gap.
  Future<void> adjustGap(double newGap) async {
    await _applyGap(newGap);
  }

  /// Increment gap by the default step size.
  Future<void> incrementGap() async {
    await _applyGap(state.gap + AppConstants.defaultLevelStep);
  }

  /// Decrement gap by the default step size.
  Future<void> decrementGap() async {
    await _applyGap(state.gap - AppConstants.defaultLevelStep);
  }

  /// Clamp a level value to the valid bounds.
  double _clampLevel(double level) {
    final clamped = level.clamp(
      AppConstants.minLevelBound,
      AppConstants.maxLevelBound,
    );
    return _roundToTenth(clamped.toDouble());
  }

  Future<void> _applyGap(double newGap) async {
    final maxGap = _roundToTenth(AppConstants.maxLevelBound - state.start);
    final validGap = _roundToTenth(newGap.clamp(0.0, maxGap).toDouble());
    final newEnd = _roundToTenth(state.start + validGap);
    final validEnd = _clampLevel(newEnd);
    final effectiveGap = _roundToTenth(validEnd - state.start);

    emit(state.copyWith(gap: effectiveGap, end: validEnd));
    await _persist();
  }

  double _roundToTenth(double value) {
    return (value * 10).roundToDouble() / 10.0;
  }
}
