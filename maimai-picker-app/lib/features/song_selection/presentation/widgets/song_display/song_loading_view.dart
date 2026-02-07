import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';

class SongLoadingView extends StatelessWidget {
  const SongLoadingView({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: SizedBox(
        width: 56,
        height: 56,
        child: CircularProgressIndicator(
          strokeWidth: 4,
          color: AppColors.accentPrimary,
        ),
      ),
    );
  }
}
