import 'package:flutter/material.dart';

class AppColors {
  AppColors._();

  static const Color background = Color(0xFF0A0E1A);
  static const Color surface = Color(0xFF141824);
  static const Color surfaceElevated = Color(0xFF1C2133);

  static const Color accentPrimary = Color(0xFF00E5FF);
  static const Color accentSecondary = Color(0xFFFF2D78);
  static const Color accentTertiary = Color(0xFF76FF03);

  static const Color textPrimary = Color(0xF2FFFFFF);
  static const Color textSecondary = Color(0x99FFFFFF);
  static const Color textMuted = Color(0x59FFFFFF);

  static const Color error = Color(0xFFFF4D67);
  static const Color success = Color(0xFF35E88A);

  static const Color badgeGold = Color(0xFFFFD166);
  static const Color badgeOrange = Color(0xFFFFA84F);

  static const Map<String, Color> difficultyColors = {
    'BASIC': Color(0xFF69C36D),
    'ADVANCED': Color(0xFFF4C430),
    'EXPERT': Color(0xFFFF6B8A),
    'MASTER': Color(0xFF9B59B6),
    'RE:MASTER': Color(0xFFF2F4FF),
  };

  static const Map<String, Color> rankColors = {
    'SSS+': badgeGold,
    'SSS': badgeGold,
    'SS+': badgeOrange,
    'SS': badgeOrange,
    'S+': accentSecondary,
    'S': accentSecondary,
    'AAA': success,
    'AA': success,
    'A': success,
  };
}
