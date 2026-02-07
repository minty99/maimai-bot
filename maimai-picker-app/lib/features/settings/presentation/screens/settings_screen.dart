import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../../core/theme/app_colors.dart';
import '../../../../core/theme/app_spacing.dart';
import '../../../../core/theme/app_typography.dart';
import '../../bloc/settings/settings_cubit.dart';
import '../../bloc/settings/settings_state.dart';
import '../widgets/display_options_section.dart';
import '../widgets/server_connection_section.dart';
import '../widgets/settings_action_buttons.dart';

class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key});

  static const String routeName = '/settings';

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: AppColors.background,
      body: SafeArea(
        child: BlocBuilder<SettingsCubit, SettingsState>(
          builder: (context, state) {
            return ListView(
              padding: const EdgeInsets.fromLTRB(
                AppSpacing.screenPadding,
                AppSpacing.md,
                AppSpacing.screenPadding,
                AppSpacing.xl,
              ),
              children: [
                Row(
                  children: [
                    IconButton(
                      onPressed: () => Navigator.of(context).pop(),
                      style: IconButton.styleFrom(
                        backgroundColor: AppColors.surfaceElevated,
                        foregroundColor: AppColors.accentPrimary,
                      ),
                      icon: const Icon(Icons.arrow_back_rounded),
                    ),
                    const SizedBox(width: AppSpacing.sm),
                    Text(
                      'Settings',
                      style: AppTypography.textTheme.headlineMedium?.copyWith(
                        color: AppColors.textPrimary,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: AppSpacing.xl),
                const ServerConnectionSection(),
                const SizedBox(height: AppSpacing.xxl),
                DisplayOptionsSection(state: state),
                const SizedBox(height: AppSpacing.xxl),
                SettingsActionButtons(
                  onResetPressed: () async {
                    await context.read<SettingsCubit>().resetServerUrls();
                    if (context.mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(
                          content: Text('Reset to default server URLs'),
                        ),
                      );
                    }
                  },
                  version: 'Version 1.0.0',
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}
