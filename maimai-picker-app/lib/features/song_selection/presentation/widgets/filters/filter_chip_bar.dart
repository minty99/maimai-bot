import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_spacing.dart';
import '../../../../../core/theme/app_typography.dart';

class FilterChipBar extends StatelessWidget {
  const FilterChipBar({
    super.key,
    required this.chartTypeLabel,
    required this.difficultyLabel,
    required this.versionLabel,
    required this.onChartTypeTap,
    required this.onDifficultyTap,
    required this.onVersionTap,
    required this.isVersionLoading,
  });

  final String chartTypeLabel;
  final String difficultyLabel;
  final String versionLabel;
  final VoidCallback onChartTypeTap;
  final VoidCallback onDifficultyTap;
  final VoidCallback onVersionTap;
  final bool isVersionLoading;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: _FilterChip(
            icon: Icons.tune,
            label: chartTypeLabel,
            onTap: onChartTypeTap,
          ),
        ),
        const SizedBox(width: AppSpacing.xs + 2), // 6px
        Expanded(
          child: _FilterChip(
            icon: Icons.flash_on_rounded,
            label: difficultyLabel,
            onTap: onDifficultyTap,
          ),
        ),
        const SizedBox(width: AppSpacing.xs + 2), // 6px
        Expanded(
          child: _FilterChip(
            icon: Icons.history_toggle_off,
            label: isVersionLoading ? '...' : versionLabel,
            onTap: onVersionTap,
          ),
        ),
      ],
    );
  }
}

class _FilterChip extends StatelessWidget {
  const _FilterChip({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(20),
        child: Ink(
          height: 34,
          padding: const EdgeInsets.symmetric(horizontal: AppSpacing.sm),
          decoration: BoxDecoration(
            color: AppColors.surface.withValues(alpha: 0.72),
            borderRadius: BorderRadius.circular(20),
            border: Border.all(
              color: AppColors.accentPrimary.withValues(alpha: 0.65),
            ),
            boxShadow: [
              BoxShadow(
                color: AppColors.accentPrimary.withValues(alpha: 0.15),
                blurRadius: 10,
                spreadRadius: 0,
              ),
            ],
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(icon, size: 14, color: AppColors.accentPrimary),
              const SizedBox(width: 3),
              Flexible(
                child: Text(
                  label,
                  textAlign: TextAlign.center,
                  overflow: TextOverflow.ellipsis,
                  style: AppTypography.textTheme.labelSmall?.copyWith(
                    color: AppColors.textPrimary,
                    fontSize: 11,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
