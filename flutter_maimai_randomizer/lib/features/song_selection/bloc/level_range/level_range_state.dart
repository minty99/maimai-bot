import 'package:equatable/equatable.dart';

/// State for LevelRangeCubit
class LevelRangeState extends Equatable {
  const LevelRangeState({
    required this.start,
    required this.end,
    required this.gap,
  });

  /// Start of the level range (inclusive).
  final double start;

  /// End of the level range (inclusive).
  final double end;

  /// Gap between start and end.
  final double gap;

  /// Creates a copy of this state with optional field overrides.
  LevelRangeState copyWith({double? start, double? end, double? gap}) {
    return LevelRangeState(
      start: start ?? this.start,
      end: end ?? this.end,
      gap: gap ?? this.gap,
    );
  }

  @override
  List<Object?> get props => [start, end, gap];
}
