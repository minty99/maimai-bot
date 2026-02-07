import 'package:flutter/material.dart';

import '../../../../core/theme/app_colors.dart';
import '../../../../core/theme/app_spacing.dart';
import '../../../../core/theme/app_typography.dart';

class SettingsActionButtons extends StatelessWidget {
  const SettingsActionButtons({
    super.key,
    required this.onResetPressed,
    required this.version,
  });

  final VoidCallback onResetPressed;
  final String version;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          height: 52,
          child: OutlinedButton(
            onPressed: onResetPressed,
            style: OutlinedButton.styleFrom(
              foregroundColor: AppColors.accentSecondary,
              backgroundColor: AppColors.accentSecondary.withValues(
                alpha: 0.08,
              ),
              side: const BorderSide(
                color: AppColors.accentSecondary,
                width: 1.5,
              ),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            child: Text(
              'RESET TO DEFAULT',
              style: AppTypography.textTheme.labelLarge?.copyWith(
                color: AppColors.accentSecondary,
                letterSpacing: 1.1,
              ),
            ),
          ),
        ),
        const SizedBox(height: AppSpacing.xl),
        Center(
          child: Text(
            version,
            style: AppTypography.textTheme.bodySmall?.copyWith(
              color: AppColors.textMuted,
            ),
          ),
        ),
      ],
    );
  }
}
