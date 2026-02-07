import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_typography.dart';

class SongErrorView extends StatelessWidget {
  const SongErrorView({super.key, required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(
            Icons.error_outline_rounded,
            size: 78,
            color: AppColors.error,
          ),
          const SizedBox(height: 12),
          Text(
            'Request failed',
            style: AppTypography.textTheme.titleLarge?.copyWith(
              color: AppColors.error,
            ),
          ),
          const SizedBox(height: 6),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20),
            child: Text(
              message,
              textAlign: TextAlign.center,
              style: AppTypography.textTheme.bodySmall,
            ),
          ),
        ],
      ),
    );
  }
}
