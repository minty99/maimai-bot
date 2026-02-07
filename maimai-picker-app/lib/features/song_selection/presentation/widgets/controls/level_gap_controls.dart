import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_spacing.dart';
import '../../../../../core/theme/app_typography.dart';

/// Compact LV / GAP stepper controls designed for narrow portrait screens.
///
/// Layout:  [ LV ◀ 12.5 ▶ ]  [ GAP ◀ 0.5 ▶ ]
///
/// Each cluster: label(22px) + btn(34) + value(flex) + btn(34) + padding.
/// Total per cluster ≈ 130px at minimum → two clusters + 6px gap = 266px.
/// Leaves ~77px for a 40px settings icon comfortably on a 375pt screen.
class LevelGapControls extends StatelessWidget {
  const LevelGapControls({
    super.key,
    required this.levelText,
    required this.gapText,
    required this.onLevelDecrement,
    required this.onLevelIncrement,
    required this.onGapDecrement,
    required this.onGapIncrement,
  });

  final String levelText;
  final String gapText;
  final VoidCallback onLevelDecrement;
  final VoidCallback onLevelIncrement;
  final VoidCallback onGapDecrement;
  final VoidCallback onGapIncrement;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: _AdjustCluster(
            label: 'LV',
            value: levelText,
            onDecrement: onLevelDecrement,
            onIncrement: onLevelIncrement,
          ),
        ),
        const SizedBox(width: AppSpacing.xs + 2), // 6px
        Expanded(
          child: _AdjustCluster(
            label: 'GAP',
            value: gapText,
            onDecrement: onGapDecrement,
            onIncrement: onGapIncrement,
          ),
        ),
      ],
    );
  }
}

class _AdjustCluster extends StatelessWidget {
  const _AdjustCluster({
    required this.label,
    required this.value,
    required this.onDecrement,
    required this.onIncrement,
  });

  final String label;
  final String value;
  final VoidCallback onDecrement;
  final VoidCallback onIncrement;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 40,
      padding: const EdgeInsets.only(left: 6, right: 2),
      decoration: BoxDecoration(
        color: AppColors.surface.withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: AppColors.accentPrimary.withValues(alpha: 0.35),
        ),
      ),
      child: Row(
        children: [
          Text(
            label,
            style: AppTypography.textTheme.labelSmall?.copyWith(
              color: AppColors.textMuted,
              fontSize: 10,
              letterSpacing: 0.5,
            ),
          ),
          const SizedBox(width: 2),
          _AdjustButton(icon: Icons.remove_rounded, onTap: onDecrement),
          Expanded(
            child: Center(
              child: Text(
                value,
                style: AppTypography.numeric.copyWith(
                  color: AppColors.accentPrimary,
                  fontSize: 16,
                ),
              ),
            ),
          ),
          _AdjustButton(icon: Icons.add_rounded, onTap: onIncrement),
        ],
      ),
    );
  }
}

class _AdjustButton extends StatelessWidget {
  const _AdjustButton({required this.icon, required this.onTap});

  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(9),
        child: Ink(
          width: 34,
          height: 34,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(9),
            border: Border.all(
              color: AppColors.accentSecondary.withValues(alpha: 0.4),
            ),
            color: AppColors.surfaceElevated.withValues(alpha: 0.7),
          ),
          child: Icon(icon, color: AppColors.accentSecondary, size: 18),
        ),
      ),
    );
  }
}
