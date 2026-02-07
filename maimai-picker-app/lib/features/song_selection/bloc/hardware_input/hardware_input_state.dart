/// State for HardwareInputCubit
///
/// Note: We don't use Equatable here because we want BlocListener to be
/// notified every time a state is emitted, even if it's the same type.
/// This is necessary for repeated button presses to trigger actions.
sealed class HardwareInputState {
  const HardwareInputState();
}

/// Initial state
class HardwareInputInitial extends HardwareInputState {
  const HardwareInputInitial();
}

/// Listening state (hardware input listeners are active)
class HardwareInputListening extends HardwareInputState {
  const HardwareInputListening();
}

/// Increment range state (volume up on Android/iOS, arrow up on macOS)
class IncrementRangeState extends HardwareInputState {
  /// Unique timestamp to ensure each emit is treated as a new state
  final int timestamp;

  IncrementRangeState() : timestamp = DateTime.now().microsecondsSinceEpoch;
}

/// Decrement range state (volume down on Android/iOS, arrow down on macOS)
class DecrementRangeState extends HardwareInputState {
  /// Unique timestamp to ensure each emit is treated as a new state
  final int timestamp;

  DecrementRangeState() : timestamp = DateTime.now().microsecondsSinceEpoch;
}

/// Trigger random state (both volume buttons on Android/iOS, space/enter on macOS)
class TriggerRandomState extends HardwareInputState {
  /// Unique timestamp to ensure each emit is treated as a new state
  final int timestamp;

  TriggerRandomState() : timestamp = DateTime.now().microsecondsSinceEpoch;
}

/// Error state
class HardwareInputError extends HardwareInputState {
  final String message;

  const HardwareInputError(this.message);
}
