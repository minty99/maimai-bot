import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../../core/theme/app_colors.dart';
import '../../../../core/theme/app_spacing.dart';
import '../../../../core/theme/app_typography.dart';
import '../../bloc/settings/settings_cubit.dart';
import '../../bloc/settings/settings_state.dart';

class DisplayOptionsSection extends StatelessWidget {
  const DisplayOptionsSection({super.key, required this.state});

  final SettingsState state;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'DISPLAY',
          style: AppTypography.textTheme.labelLarge?.copyWith(
            color: AppColors.accentPrimary,
            letterSpacing: 1.8,
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        Container(
          decoration: BoxDecoration(
            color: AppColors.surfaceElevated,
            borderRadius: BorderRadius.circular(18),
            border: Border.all(
              color: AppColors.accentPrimary.withValues(alpha: 0.62),
            ),
            boxShadow: [
              BoxShadow(
                color: AppColors.accentPrimary.withValues(alpha: 0.14),
                blurRadius: 18,
                spreadRadius: -8,
              ),
            ],
          ),
          child: Column(
            children: [
              _NeonSwitchTile(
                title: 'Show Level',
                subtitle: 'Display level text like 13+',
                value: state.showLevel,
                onChanged: (value) {
                  context.read<SettingsCubit>().updateShowLevel(value);
                },
              ),
              const Divider(height: 1, color: Color(0x3337EFFF)),
              _NeonSwitchTile(
                title: 'Show User Level',
                subtitle: 'Display user level label like (A)',
                value: state.showUserLevel,
                onChanged: state.showLevel
                    ? (value) {
                        context.read<SettingsCubit>().updateShowUserLevel(
                          value,
                        );
                      }
                    : null,
              ),
            ],
          ),
        ),
        const SizedBox(height: AppSpacing.xl),
        Container(
          padding: const EdgeInsets.all(AppSpacing.lg),
          decoration: BoxDecoration(
            color: AppColors.surfaceElevated,
            borderRadius: BorderRadius.circular(18),
            border: Border.all(
              color: AppColors.accentTertiary.withValues(alpha: 0.45),
            ),
            boxShadow: [
              BoxShadow(
                color: AppColors.accentTertiary.withValues(alpha: 0.12),
                blurRadius: 18,
                spreadRadius: -8,
              ),
            ],
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Icon(
                    Icons.leaderboard_rounded,
                    size: 22,
                    color: AppColors.accentTertiary,
                  ),
                  const SizedBox(width: AppSpacing.sm),
                  Text(
                    'User Level Guide',
                    style: AppTypography.textTheme.titleMedium?.copyWith(
                      color: AppColors.textPrimary,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: AppSpacing.sm),
              Text(
                'Shown next to internal level as level 13.7 (A).',
                style: AppTypography.textTheme.bodyMedium,
              ),
              const SizedBox(height: AppSpacing.xs),
              Text(
                'Ranks from highest to lowest:',
                style: AppTypography.textTheme.bodySmall,
              ),
              const SizedBox(height: AppSpacing.md),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(
                  horizontal: AppSpacing.md,
                  vertical: AppSpacing.sm,
                ),
                decoration: BoxDecoration(
                  color: AppColors.surface,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: AppColors.accentTertiary.withValues(alpha: 0.5),
                  ),
                ),
                child: Text(
                  'S  >  A  >  B  >  C  >  D  >  E  >  F',
                  textAlign: TextAlign.center,
                  style: AppTypography.numeric.copyWith(
                    color: AppColors.accentTertiary,
                    fontSize: 16,
                    letterSpacing: 1.2,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _NeonSwitchTile extends StatelessWidget {
  const _NeonSwitchTile({
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    final disabled = onChanged == null;
    return SwitchListTile(
      contentPadding: const EdgeInsets.symmetric(horizontal: AppSpacing.md),
      title: Text(
        title,
        style: AppTypography.textTheme.titleMedium?.copyWith(
          color: disabled ? AppColors.textMuted : AppColors.textPrimary,
        ),
      ),
      subtitle: Text(
        subtitle,
        style: AppTypography.textTheme.bodySmall?.copyWith(
          color: disabled ? AppColors.textMuted : AppColors.textSecondary,
        ),
      ),
      value: value,
      onChanged: onChanged,
      activeThumbColor: AppColors.background,
      activeTrackColor: AppColors.accentPrimary,
      inactiveThumbColor: AppColors.textMuted,
      inactiveTrackColor: AppColors.surface,
    );
  }
}
