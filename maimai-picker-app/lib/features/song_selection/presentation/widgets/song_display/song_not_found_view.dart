import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_typography.dart';

class SongNotFoundView extends StatelessWidget {
  const SongNotFoundView({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.search_off_rounded,
            size: 80,
            color: AppColors.accentSecondary.withValues(alpha: 0.9),
          ),
          const SizedBox(height: 12),
          Text(
            'No songs in this range',
            style: AppTypography.textTheme.titleLarge?.copyWith(
              color: AppColors.textPrimary,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            'Widen LV/GAP or adjust filters.',
            style: AppTypography.textTheme.bodySmall,
          ),
        ],
      ),
    );
  }
}
