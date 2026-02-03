import 'package:flutter/material.dart';

import '../constants/app_constants.dart';

/// Application theme configuration with Material 3 design.
///
/// Optimized for:
/// - Glove-friendly large touch targets
/// - High contrast for arcade environment visibility
/// - maimai-inspired color palette
class AppTheme {
  AppTheme._();

  // ─────────────────────────────────────────────────────────────────────────
  // Color Configuration
  // ─────────────────────────────────────────────────────────────────────────

  /// Primary seed color - maimai signature pink/magenta.
  static const Color _seedColor = Color(0xFFE91E63);

  /// Dark background for arcade-like ambiance.
  static const Color _darkBackground = Color(0xFF0D0D0D);

  /// Surface color with subtle contrast.
  static const Color _darkSurface = Color(0xFF1A1A1A);

  // ─────────────────────────────────────────────────────────────────────────
  // Theme Data
  // ─────────────────────────────────────────────────────────────────────────

  /// Dark theme - primary theme for arcade environment.
  static ThemeData get darkTheme {
    final colorScheme = ColorScheme.fromSeed(
      seedColor: _seedColor,
      brightness: Brightness.dark,
      surface: _darkSurface,
    );

    return ThemeData(
      useMaterial3: true,
      colorScheme: colorScheme,
      scaffoldBackgroundColor: _darkBackground,

      // ─────────────────────────────────────────────────────────────────────
      // Typography - Large, readable text for arcade visibility
      // ─────────────────────────────────────────────────────────────────────
      textTheme: const TextTheme(
        displayLarge: TextStyle(
          fontSize: 48,
          fontWeight: FontWeight.bold,
          letterSpacing: -1.5,
        ),
        displayMedium: TextStyle(
          fontSize: 36,
          fontWeight: FontWeight.bold,
          letterSpacing: -0.5,
        ),
        headlineLarge: TextStyle(fontSize: 28, fontWeight: FontWeight.w600),
        headlineMedium: TextStyle(fontSize: 24, fontWeight: FontWeight.w600),
        titleLarge: TextStyle(fontSize: 20, fontWeight: FontWeight.w500),
        bodyLarge: TextStyle(fontSize: 18, fontWeight: FontWeight.normal),
        bodyMedium: TextStyle(fontSize: 16, fontWeight: FontWeight.normal),
        labelLarge: TextStyle(
          fontSize: 16,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),

      // ─────────────────────────────────────────────────────────────────────
      // Button Themes - Large touch targets for gloved hands
      // ─────────────────────────────────────────────────────────────────────
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          minimumSize: Size(
            AppConstants.minTouchTargetSize,
            AppConstants.largeButtonHeight,
          ),
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
          textStyle: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
        ),
      ),

      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: Size(
            AppConstants.minTouchTargetSize,
            AppConstants.largeButtonHeight,
          ),
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
          textStyle: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
        ),
      ),

      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: Size(
            AppConstants.minTouchTargetSize,
            AppConstants.largeButtonHeight,
          ),
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
          textStyle: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          side: BorderSide(color: colorScheme.outline, width: 2),
        ),
      ),

      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(
          minimumSize: const Size(
            AppConstants.minTouchTargetSize,
            AppConstants.minTouchTargetSize,
          ),
          iconSize: 32,
        ),
      ),

      // ─────────────────────────────────────────────────────────────────────
      // Card & Surface styling
      // ─────────────────────────────────────────────────────────────────────
      cardTheme: CardThemeData(
        elevation: 4,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
        color: _darkSurface,
      ),

      // ─────────────────────────────────────────────────────────────────────
      // AppBar
      // ─────────────────────────────────────────────────────────────────────
      appBarTheme: AppBarTheme(
        backgroundColor: _darkBackground,
        elevation: 0,
        centerTitle: true,
        titleTextStyle: TextStyle(
          color: colorScheme.onSurface,
          fontSize: 22,
          fontWeight: FontWeight.w600,
        ),
      ),

      // ─────────────────────────────────────────────────────────────────────
      // Slider - For level range selection
      // ─────────────────────────────────────────────────────────────────────
      sliderTheme: SliderThemeData(
        thumbShape: const RoundSliderThumbShape(
          enabledThumbRadius: 16, // Large thumb for gloved interaction
        ),
        overlayShape: const RoundSliderOverlayShape(overlayRadius: 28),
        trackHeight: 8,
        activeTrackColor: colorScheme.primary,
        inactiveTrackColor: colorScheme.surfaceContainerHighest,
        thumbColor: colorScheme.primary,
        overlayColor: colorScheme.primary.withValues(alpha: 0.2),
      ),
    );
  }

  /// Light theme - alternative for bright environments.
  static ThemeData get lightTheme {
    final colorScheme = ColorScheme.fromSeed(
      seedColor: _seedColor,
      brightness: Brightness.light,
    );

    return ThemeData(
      useMaterial3: true,
      colorScheme: colorScheme,
      // Most apps will use dark theme; light theme follows same patterns
      // with Material 3 defaults for now.
    );
  }
}
