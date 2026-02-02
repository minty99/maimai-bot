import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:volume_listener/volume_listener.dart';

import 'hardware_input_state.dart';

/// Cubit for handling cross-platform hardware input
/// - Android: volume buttons (up/down) and simultaneous press for trigger
/// - iOS/macOS: keyboard arrows (up/down) and space/enter for trigger
class HardwareInputCubit extends Cubit<HardwareInputState> {
  HardwareInputCubit() : super(const HardwareInputInitial());

  // Android volume button state tracking
  bool _volumeUpPressed = false;
  bool _volumeDownPressed = false;
  Timer? _simultaneousPressTimer;
  final Duration _simultaneousPressWindow = const Duration(milliseconds: 200);

  // iOS/macOS keyboard listener callback
  late final KeyEventCallback _keyboardListener;

  /// Initialize hardware input listeners based on platform
  Future<void> initialize() async {
    try {
      if (Platform.isAndroid) {
        _initializeAndroidVolumeListener();
      } else if (Platform.isIOS || Platform.isMacOS) {
        _initializeIOSMacOSKeyboardListener();
      }
      emit(const HardwareInputListening());
    } catch (e) {
      emit(HardwareInputError('Failed to initialize hardware input: $e'));
    }
  }

  /// Initialize Android volume button listener
  void _initializeAndroidVolumeListener() {
    // Listen to volume button events using VolumeListener
    // The volume_listener package provides addListener for volume key events
    VolumeListener.addListener((VolumeKey event) {
      switch (event) {
        case VolumeKey.up:
          _handleVolumeUp();
          break;
        case VolumeKey.down:
          _handleVolumeDown();
          break;
        case VolumeKey.capture:
          // iOS 17.2+ hardware camera capture button - ignore for now
          break;
      }
    });
  }

  /// Handle volume up button press on Android
  void _handleVolumeUp() {
    _volumeUpPressed = true;

    // Check if both buttons are pressed simultaneously
    if (_volumeDownPressed) {
      _cancelSimultaneousPressTimer();
      _triggerSimultaneousPress();
    } else {
      // Set a timer to detect if the other button is pressed
      _simultaneousPressTimer = Timer(_simultaneousPressWindow, () {
        if (_volumeUpPressed && !_volumeDownPressed) {
          emit(const HardwareInputListening());
        }
        _volumeUpPressed = false;
      });
    }
  }

  /// Handle volume down button press on Android
  void _handleVolumeDown() {
    _volumeDownPressed = true;

    // Check if both buttons are pressed simultaneously
    if (_volumeUpPressed) {
      _cancelSimultaneousPressTimer();
      _triggerSimultaneousPress();
    } else {
      // Set a timer to detect if the other button is pressed
      _simultaneousPressTimer = Timer(_simultaneousPressWindow, () {
        if (_volumeDownPressed && !_volumeUpPressed) {
          emit(const HardwareInputListening());
        }
        _volumeDownPressed = false;
      });
    }
  }

  /// Handle simultaneous press of both volume buttons
  void _triggerSimultaneousPress() {
    _volumeUpPressed = false;
    _volumeDownPressed = false;
    emit(const HardwareInputListening());
  }

  /// Cancel the simultaneous press timer
  void _cancelSimultaneousPressTimer() {
    _simultaneousPressTimer?.cancel();
    _simultaneousPressTimer = null;
  }

  /// Initialize iOS/macOS keyboard listener
  void _initializeIOSMacOSKeyboardListener() {
    _keyboardListener = (KeyEvent event) {
      if (event is KeyDownEvent) {
        final logicalKey = event.logicalKey;

        if (logicalKey == LogicalKeyboardKey.arrowUp) {
          emit(const HardwareInputListening());
          return true;
        } else if (logicalKey == LogicalKeyboardKey.arrowDown) {
          emit(const HardwareInputListening());
          return true;
        } else if (logicalKey == LogicalKeyboardKey.space ||
            logicalKey == LogicalKeyboardKey.enter) {
          emit(const HardwareInputListening());
          return true;
        }
      }
      return false;
    };

    HardwareKeyboard.instance.addHandler(_keyboardListener);
  }

  @override
  Future<void> close() async {
    _cancelSimultaneousPressTimer();
    if (Platform.isIOS || Platform.isMacOS) {
      HardwareKeyboard.instance.removeHandler(_keyboardListener);
    }
    VolumeListener.removeListener();
    return super.close();
  }
}
