import 'package:equatable/equatable.dart';

/// Events emitted by HardwareInputCubit
sealed class HardwareInputEvent extends Equatable {
  const HardwareInputEvent();

  @override
  List<Object?> get props => [];
}

/// Increment range event (volume up on Android, arrow up on iOS/macOS)
class IncrementRange extends HardwareInputEvent {
  const IncrementRange();
}

/// Decrement range event (volume down on Android, arrow down on iOS/macOS)
class DecrementRange extends HardwareInputEvent {
  const DecrementRange();
}

/// Trigger random event (both volume buttons on Android, space/enter on iOS/macOS)
class TriggerRandom extends HardwareInputEvent {
  const TriggerRandom();
}
