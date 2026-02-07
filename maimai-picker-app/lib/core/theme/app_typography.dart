import 'package:flutter/material.dart';

import 'app_colors.dart';

class AppTypography {
  AppTypography._();

  static const String displayFamily = 'Rajdhani';
  static const String bodyFamily = 'Exo 2';

  static TextTheme textTheme = const TextTheme(
    displayLarge: TextStyle(
      fontFamily: displayFamily,
      fontSize: 48,
      fontWeight: FontWeight.w700,
      letterSpacing: -1.2,
      color: AppColors.textPrimary,
    ),
    displayMedium: TextStyle(
      fontFamily: displayFamily,
      fontSize: 36,
      fontWeight: FontWeight.w700,
      letterSpacing: -0.8,
      color: AppColors.textPrimary,
    ),
    headlineLarge: TextStyle(
      fontFamily: displayFamily,
      fontSize: 28,
      fontWeight: FontWeight.w700,
      letterSpacing: -0.4,
      color: AppColors.textPrimary,
    ),
    headlineMedium: TextStyle(
      fontFamily: displayFamily,
      fontSize: 22,
      fontWeight: FontWeight.w700,
      letterSpacing: -0.2,
      color: AppColors.textPrimary,
    ),
    titleLarge: TextStyle(
      fontFamily: bodyFamily,
      fontSize: 20,
      fontWeight: FontWeight.w700,
      color: AppColors.textPrimary,
    ),
    titleMedium: TextStyle(
      fontFamily: bodyFamily,
      fontSize: 16,
      fontWeight: FontWeight.w700,
      color: AppColors.textPrimary,
    ),
    titleSmall: TextStyle(
      fontFamily: bodyFamily,
      fontSize: 14,
      fontWeight: FontWeight.w700,
      color: AppColors.textSecondary,
    ),
    bodyLarge: TextStyle(
      fontFamily: bodyFamily,
      fontSize: 16,
      fontWeight: FontWeight.w500,
      color: AppColors.textPrimary,
    ),
    bodyMedium: TextStyle(
      fontFamily: bodyFamily,
      fontSize: 14,
      fontWeight: FontWeight.w500,
      color: AppColors.textSecondary,
    ),
    bodySmall: TextStyle(
      fontFamily: bodyFamily,
      fontSize: 12,
      fontWeight: FontWeight.w500,
      color: AppColors.textMuted,
    ),
    labelLarge: TextStyle(
      fontFamily: displayFamily,
      fontSize: 14,
      fontWeight: FontWeight.w700,
      letterSpacing: 1.0,
      color: AppColors.textPrimary,
    ),
    labelMedium: TextStyle(
      fontFamily: displayFamily,
      fontSize: 12,
      fontWeight: FontWeight.w700,
      letterSpacing: 0.8,
      color: AppColors.textSecondary,
    ),
    labelSmall: TextStyle(
      fontFamily: displayFamily,
      fontSize: 11,
      fontWeight: FontWeight.w700,
      letterSpacing: 0.8,
      color: AppColors.textMuted,
    ),
  );

  static TextStyle get numeric => const TextStyle(
    fontFamily: displayFamily,
    fontWeight: FontWeight.w700,
    letterSpacing: -0.2,
    color: AppColors.textPrimary,
  );
}
