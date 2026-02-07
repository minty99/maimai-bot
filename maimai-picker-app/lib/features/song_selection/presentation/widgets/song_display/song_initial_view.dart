import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_typography.dart';

class SongInitialView extends StatelessWidget {
  const SongInitialView({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 132,
            height: 132,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(24),
              color: AppColors.surfaceElevated.withValues(alpha: 0.85),
              border: Border.all(
                color: AppColors.accentPrimary.withValues(alpha: 0.5),
              ),
            ),
            child: const Icon(
              Icons.music_note_rounded,
              size: 58,
              color: AppColors.accentPrimary,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            'Press RANDOM or shake.',
            style: AppTypography.textTheme.titleMedium,
          ),
          const SizedBox(height: 6),
          Text(
            'Volume keys / arrows also work.',
            style: AppTypography.textTheme.bodySmall,
          ),
        ],
      ),
    );
  }
}
