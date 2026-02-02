import 'package:equatable/equatable.dart';

/// State for HardwareInputCubit
sealed class HardwareInputState extends Equatable {
  const HardwareInputState();

  @override
  List<Object?> get props => [];
}

/// Initial state
class HardwareInputInitial extends HardwareInputState {
  const HardwareInputInitial();
}

/// Listening state (hardware input listeners are active)
class HardwareInputListening extends HardwareInputState {
  const HardwareInputListening();
}

/// Increment range state (volume up on Android, arrow up on iOS/macOS)
class IncrementRangeState extends HardwareInputState {
  const IncrementRangeState();
}

/// Decrement range state (volume down on Android, arrow down on iOS/macOS)
class DecrementRangeState extends HardwareInputState {
  const DecrementRangeState();
}

/// Trigger random state (both volume buttons on Android, space/enter on iOS/macOS)
class TriggerRandomState extends HardwareInputState {
  const TriggerRandomState();
}

/// Error state
class HardwareInputError extends HardwareInputState {
  final String message;

  const HardwareInputError(this.message);

  @override
  List<Object?> get props => [message];
}
