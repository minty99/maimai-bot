import 'package:flutter/material.dart';

import '../constants/app_constants.dart';
import 'app_colors.dart';
import 'app_spacing.dart';
import 'app_typography.dart';

/// Application theme configuration with Material 3 design.
///
/// Optimized for:
/// - Glove-friendly large touch targets
/// - High contrast for arcade environment visibility
/// - maimai-inspired color palette
class AppTheme {
  AppTheme._();

  // ─────────────────────────────────────────────────────────────────────────
  // Theme Data
  // ─────────────────────────────────────────────────────────────────────────

  /// Dark theme - primary theme for arcade environment.
  static ThemeData get darkTheme {
    const colorScheme = ColorScheme(
      brightness: Brightness.dark,
      primary: AppColors.accentPrimary,
      onPrimary: Color(0xFF00131A),
      secondary: AppColors.accentSecondary,
      onSecondary: Color(0xFF2E0012),
      tertiary: AppColors.accentTertiary,
      onTertiary: Color(0xFF112B00),
      error: AppColors.error,
      onError: Color(0xFF2A0006),
      surface: AppColors.surface,
      onSurface: AppColors.textPrimary,
      onSurfaceVariant: AppColors.textSecondary,
      outline: Color(0x6652C3D8),
      outlineVariant: Color(0x3346A3B9),
      shadow: Colors.black,
      scrim: Colors.black,
      inverseSurface: Color(0xFFF3F7FF),
      onInverseSurface: Color(0xFF101521),
      inversePrimary: Color(0xFF005B66),
      surfaceContainerHighest: AppColors.surfaceElevated,
    );

    return ThemeData(
      useMaterial3: true,
      colorScheme: colorScheme,
      scaffoldBackgroundColor: AppColors.background,
      textTheme: AppTypography.textTheme,

      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: AppColors.accentPrimary,
          foregroundColor: colorScheme.onPrimary,
          minimumSize: const Size(
            AppConstants.minTouchTargetSize,
            AppConstants.largeButtonHeight,
          ),
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.xl,
            vertical: AppSpacing.lg,
          ),
          textStyle: AppTypography.textTheme.labelLarge,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(14),
          ),
        ),
      ),

      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: AppColors.accentPrimary,
          foregroundColor: colorScheme.onPrimary,
          minimumSize: const Size(
            AppConstants.minTouchTargetSize,
            AppConstants.largeButtonHeight,
          ),
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.xl,
            vertical: AppSpacing.lg,
          ),
          textStyle: AppTypography.textTheme.labelLarge,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(14),
          ),
        ),
      ),

      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(
            AppConstants.minTouchTargetSize,
            AppConstants.largeButtonHeight,
          ),
          foregroundColor: AppColors.textPrimary,
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.xl,
            vertical: AppSpacing.lg,
          ),
          textStyle: AppTypography.textTheme.labelLarge,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(14),
          ),
          side: const BorderSide(color: AppColors.accentPrimary, width: 1.4),
        ),
      ),

      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(
          minimumSize: const Size(
            AppConstants.minTouchTargetSize,
            AppConstants.minTouchTargetSize,
          ),
          foregroundColor: AppColors.textPrimary,
          iconSize: 28,
        ),
      ),

      cardTheme: CardThemeData(
        elevation: 0,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
        color: AppColors.surfaceElevated,
      ),

      appBarTheme: AppBarTheme(
        backgroundColor: AppColors.background,
        elevation: 0,
        centerTitle: true,
        titleTextStyle: AppTypography.textTheme.titleLarge,
      ),

      sliderTheme: SliderThemeData(
        thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 16),
        overlayShape: const RoundSliderOverlayShape(overlayRadius: 28),
        trackHeight: 8,
        activeTrackColor: AppColors.accentPrimary,
        inactiveTrackColor: AppColors.surfaceElevated,
        thumbColor: AppColors.accentSecondary,
        overlayColor: AppColors.accentPrimary.withValues(alpha: 0.22),
      ),

      chipTheme: ChipThemeData(
        backgroundColor: AppColors.surface,
        selectedColor: AppColors.surfaceElevated,
        disabledColor: AppColors.surface,
        deleteIconColor: AppColors.textSecondary,
        labelStyle: AppTypography.textTheme.labelMedium!,
        secondaryLabelStyle: AppTypography.textTheme.labelMedium!,
        brightness: Brightness.dark,
        side: const BorderSide(color: AppColors.accentPrimary, width: 1),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
      ),
    );
  }

  /// Light theme - alternative for bright environments.
  static ThemeData get lightTheme {
    return ThemeData(
      useMaterial3: true,
      colorScheme: const ColorScheme.light(),
    );
  }
}
