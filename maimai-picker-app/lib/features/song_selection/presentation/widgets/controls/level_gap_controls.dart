import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_spacing.dart';
import '../../../../../core/theme/app_typography.dart';

/// Range-first level control with prominent range display and large arrows.
///
/// Layout (two rows):
///   Row 1:  [ ◀ ]   12.5 ~ 13.0   [ ▶ ]       ← range + level arrows
///   Row 2:             GAP [ ◀ 0.5 ▶ ]         ← secondary GAP stepper
///
/// The range ("start ~ end") is the hero element — immediately readable.
/// Level arrows are large (42px) since level changes are frequent.
/// GAP controls are compact since gap changes are infrequent.
class LevelGapControls extends StatelessWidget {
  const LevelGapControls({
    super.key,
    required this.rangeStart,
    required this.rangeEnd,
    required this.gapText,
    required this.onLevelDecrement,
    required this.onLevelIncrement,
    required this.onGapDecrement,
    required this.onGapIncrement,
  });

  final String rangeStart;
  final String rangeEnd;
  final String gapText;
  final VoidCallback onLevelDecrement;
  final VoidCallback onLevelIncrement;
  final VoidCallback onGapDecrement;
  final VoidCallback onGapIncrement;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // ── Primary row: [ - ]  12.5 ~ 13.0  [ + ] ──
        Row(
          children: [
            _LevelArrow(icon: Icons.remove_rounded, onTap: onLevelDecrement),
            Expanded(
              child: Center(
                child: Text.rich(
                  TextSpan(
                    children: [
                      TextSpan(
                        text: rangeStart,
                        style: AppTypography.numeric.copyWith(
                          color: AppColors.accentPrimary,
                          fontSize: 22,
                        ),
                      ),
                      TextSpan(
                        text: '  ~  ',
                        style: AppTypography.numeric.copyWith(
                          color: AppColors.textMuted,
                          fontSize: 16,
                        ),
                      ),
                      TextSpan(
                        text: rangeEnd,
                        style: AppTypography.numeric.copyWith(
                          color: AppColors.accentPrimary,
                          fontSize: 22,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            _LevelArrow(icon: Icons.add_rounded, onTap: onLevelIncrement),
          ],
        ),
        const SizedBox(height: AppSpacing.xs),

        // ── Secondary row: GAP [ ◀ 0.5 ▶ ] ──
        _GapStepper(
          value: gapText,
          onDecrement: onGapDecrement,
          onIncrement: onGapIncrement,
        ),
      ],
    );
  }
}

/// Large arrow button for level adjustment (frequent action, glove-friendly).
class _LevelArrow extends StatelessWidget {
  const _LevelArrow({required this.icon, required this.onTap});

  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Ink(
          width: 42,
          height: 42,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),
            color: AppColors.surface.withValues(alpha: 0.8),
            border: Border.all(
              color: AppColors.accentPrimary.withValues(alpha: 0.4),
            ),
          ),
          child: Icon(icon, color: AppColors.accentPrimary, size: 26),
        ),
      ),
    );
  }
}

/// Compact GAP stepper — small since gap changes are infrequent.
class _GapStepper extends StatelessWidget {
  const _GapStepper({
    required this.value,
    required this.onDecrement,
    required this.onIncrement,
  });

  final String value;
  final VoidCallback onDecrement;
  final VoidCallback onIncrement;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        _SmallButton(icon: Icons.remove_rounded, onTap: onDecrement),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10),
          child: Text(
            value,
            style: AppTypography.numeric.copyWith(
              color: AppColors.textSecondary,
              fontSize: 14,
            ),
          ),
        ),
        _SmallButton(icon: Icons.add_rounded, onTap: onIncrement),
      ],
    );
  }
}

/// Small button for GAP adjustment.
class _SmallButton extends StatelessWidget {
  const _SmallButton({required this.icon, required this.onTap});

  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Ink(
          width: 28,
          height: 28,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            color: AppColors.surfaceElevated.withValues(alpha: 0.7),
            border: Border.all(
              color: AppColors.textMuted.withValues(alpha: 0.4),
            ),
          ),
          child: Icon(icon, color: AppColors.textSecondary, size: 16),
        ),
      ),
    );
  }
}
