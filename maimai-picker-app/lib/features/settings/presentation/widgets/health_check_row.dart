import 'package:flutter/material.dart';

import '../../../../core/theme/app_colors.dart';
import '../../../../core/theme/app_motion.dart';
import '../../../../core/theme/app_spacing.dart';
import '../../../../core/theme/app_typography.dart';

class HealthCheckRow extends StatelessWidget {
  const HealthCheckRow({
    super.key,
    required this.isChecking,
    required this.onPressed,
    required this.healthOk,
    required this.healthMessage,
  });

  final bool isChecking;
  final VoidCallback onPressed;
  final bool? healthOk;
  final String? healthMessage;

  @override
  Widget build(BuildContext context) {
    final statusColor = healthOk == true ? AppColors.success : AppColors.error;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          height: 44,
          child: OutlinedButton(
            onPressed: isChecking ? null : onPressed,
            style: OutlinedButton.styleFrom(
              foregroundColor: AppColors.accentPrimary,
              side: BorderSide(
                color: AppColors.accentPrimary.withValues(alpha: 0.85),
              ),
              backgroundColor: AppColors.surface.withValues(alpha: 0.75),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
            ),
            child: AnimatedSwitcher(
              duration: AppMotion.fast,
              switchInCurve: AppMotion.enter,
              switchOutCurve: AppMotion.exit,
              child: isChecking
                  ? Row(
                      key: const ValueKey('checking'),
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: AppColors.accentPrimary,
                          ),
                        ),
                        const SizedBox(width: AppSpacing.sm),
                        Text(
                          'CHECKING...',
                          style: AppTypography.textTheme.labelMedium?.copyWith(
                            color: AppColors.accentPrimary,
                          ),
                        ),
                      ],
                    )
                  : Row(
                      key: const ValueKey('idle'),
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        const Icon(
                          Icons.network_check_rounded,
                          size: 18,
                          color: AppColors.accentPrimary,
                        ),
                        const SizedBox(width: AppSpacing.xs),
                        Text(
                          'HEALTH CHECK',
                          style: AppTypography.textTheme.labelMedium?.copyWith(
                            color: AppColors.accentPrimary,
                            letterSpacing: 1,
                          ),
                        ),
                      ],
                    ),
            ),
          ),
        ),
        if (healthMessage != null) ...[
          const SizedBox(height: AppSpacing.sm),
          Container(
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.md,
              vertical: AppSpacing.sm,
            ),
            decoration: BoxDecoration(
              color: statusColor.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: statusColor.withValues(alpha: 0.55)),
            ),
            child: Row(
              children: [
                Icon(
                  healthOk == true
                      ? Icons.check_circle_rounded
                      : Icons.error_rounded,
                  size: 16,
                  color: statusColor,
                ),
                const SizedBox(width: AppSpacing.xs),
                Expanded(
                  child: Text(
                    healthMessage!,
                    style: AppTypography.textTheme.bodySmall?.copyWith(
                      color: statusColor,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }
}
