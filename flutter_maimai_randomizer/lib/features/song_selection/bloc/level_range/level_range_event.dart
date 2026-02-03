import 'package:equatable/equatable.dart';

/// Events for LevelRangeCubit
sealed class LevelRangeEvent extends Equatable {
  const LevelRangeEvent();

  @override
  List<Object?> get props => [];
}

/// Update the entire range (start and end)
class UpdateRange extends LevelRangeEvent {
  final double start;
  final double end;

  const UpdateRange({required this.start, required this.end});

  @override
  List<Object?> get props => [start, end];
}

/// Increment start level by gap, adjust end to maintain gap
class IncrementStart extends LevelRangeEvent {
  const IncrementStart();
}

/// Decrement start level by gap, adjust end to maintain gap
class DecrementStart extends LevelRangeEvent {
  const DecrementStart();
}

/// Adjust the gap between start and end
class AdjustGap extends LevelRangeEvent {
  final double newGap;

  const AdjustGap(this.newGap);

  @override
  List<Object?> get props => [newGap];
}
