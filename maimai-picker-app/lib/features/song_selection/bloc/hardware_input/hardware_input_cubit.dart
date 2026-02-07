import 'dart:async';
import 'dart:developer' as developer;
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:sensors_plus/sensors_plus.dart';
import 'package:volume_button_override/volume_button_override.dart';

import 'hardware_input_state.dart';

/// Cubit for handling cross-platform hardware input
/// - Android/iOS: volume buttons (up/down) and shake gesture for trigger
/// - macOS: keyboard arrows (up/down) and space/enter for trigger
class HardwareInputCubit extends Cubit<HardwareInputState> {
  HardwareInputCubit() : super(const HardwareInputInitial());

  // Volume button controller
  VolumeButtonController? _volumeController;

  // Shake detection via accelerometer
  StreamSubscription<AccelerometerEvent>? _accelerometerSubscription;
  DateTime? _lastShakeTime;
  final Duration _shakeCooldown = const Duration(milliseconds: 500);
  final double _shakeThreshold = 2.7; // g-force threshold

  // macOS keyboard listener callback
  late final KeyEventCallback _keyboardListener;

  /// Initialize hardware input listeners based on platform
  Future<void> initialize() async {
    try {
      if (Platform.isAndroid || Platform.isIOS) {
        await _initializeVolumeListener();
        _initializeShakeListener();
      }
      if (Platform.isMacOS) {
        _initializeMacOSKeyboardListener();
      }
      emit(const HardwareInputListening());
    } catch (e) {
      emit(HardwareInputError('Failed to initialize hardware input: $e'));
    }
  }

  /// Initialize volume button listener for Android and iOS
  /// Uses volume_button_override which prevents OS volume changes
  Future<void> _initializeVolumeListener() async {
    _volumeController = VolumeButtonController();

    final volumeUpAction = ButtonAction(
      id: ButtonActionId.volumeUp,
      onAction: () => _handleVolumeUp(),
    );

    final volumeDownAction = ButtonAction(
      id: ButtonActionId.volumeDown,
      onAction: () => _handleVolumeDown(),
    );

    await _volumeController!.startListening(
      volumeUpAction: volumeUpAction,
      volumeDownAction: volumeDownAction,
    );
  }

  /// Initialize shake detection listener for Android and iOS
  void _initializeShakeListener() {
    _accelerometerSubscription = accelerometerEventStream().listen(
      (AccelerometerEvent event) {
        // Calculate g-force magnitude
        final gForce = (event.x.abs() + event.y.abs() + event.z.abs()) / 9.8;

        if (gForce > _shakeThreshold) {
          final now = DateTime.now();
          if (_lastShakeTime == null ||
              now.difference(_lastShakeTime!) > _shakeCooldown) {
            _lastShakeTime = now;
            developer.log(
              'Shake detected (g-force: ${gForce.toStringAsFixed(2)}) -> RANDOM',
              name: 'HardwareInput',
            );
            emit(TriggerRandomState());
          }
        }
      },
      onError: (error) {
        developer.log(
          'Accelerometer error: $error',
          name: 'HardwareInput',
          error: error,
        );
      },
      cancelOnError: true,
    );
  }

  /// Handle volume up button press
  void _handleVolumeUp() {
    developer.log('Volume UP pressed -> INCREMENT', name: 'HardwareInput');
    emit(IncrementRangeState());
  }

  /// Handle volume down button press
  void _handleVolumeDown() {
    developer.log('Volume DOWN pressed -> DECREMENT', name: 'HardwareInput');
    emit(DecrementRangeState());
  }

  /// Initialize macOS keyboard listener
  void _initializeMacOSKeyboardListener() {
    _keyboardListener = (KeyEvent event) {
      if (event is KeyDownEvent) {
        final logicalKey = event.logicalKey;

        if (logicalKey == LogicalKeyboardKey.arrowUp) {
          emit(IncrementRangeState());
          return true;
        } else if (logicalKey == LogicalKeyboardKey.arrowDown) {
          emit(DecrementRangeState());
          return true;
        } else if (logicalKey == LogicalKeyboardKey.space ||
            logicalKey == LogicalKeyboardKey.enter) {
          emit(TriggerRandomState());
          return true;
        }
      }
      return false;
    };

    HardwareKeyboard.instance.addHandler(_keyboardListener);
  }

  @override
  Future<void> close() async {
    await _accelerometerSubscription?.cancel();
    if (Platform.isMacOS) {
      HardwareKeyboard.instance.removeHandler(_keyboardListener);
    }
    if (Platform.isAndroid || Platform.isIOS) {
      await _volumeController?.stopListening();
    }
    return super.close();
  }
}
