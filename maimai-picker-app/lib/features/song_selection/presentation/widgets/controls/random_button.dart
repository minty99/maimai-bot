import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_motion.dart';
import '../../../../../core/theme/app_typography.dart';

/// Full-width RANDOM button designed for the bottom of the screen.
/// Positioned at the thumb zone for easy glove interaction.
/// Subtle pulse glow when idle; spinner when loading.
class RandomButton extends StatefulWidget {
  const RandomButton({
    super.key,
    required this.onPressed,
    required this.isLoading,
  });

  final VoidCallback onPressed;
  final bool isLoading;

  @override
  State<RandomButton> createState() => _RandomButtonState();
}

class _RandomButtonState extends State<RandomButton>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulseController;

  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(
      duration: AppMotion.pulse,
      vsync: this,
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _pulseController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _pulseController,
      builder: (context, child) {
        final glow = 14.0 + (_pulseController.value * 18);
        return DecoratedBox(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(14),
            boxShadow: [
              if (!widget.isLoading)
                BoxShadow(
                  color: AppColors.accentSecondary.withValues(alpha: 0.32),
                  blurRadius: glow,
                  spreadRadius: -2,
                ),
            ],
          ),
          child: SizedBox(
            width: double.infinity,
            height: 48,
            child: FilledButton(
              onPressed: widget.isLoading ? null : widget.onPressed,
              style: FilledButton.styleFrom(
                backgroundColor: AppColors.accentSecondary,
                disabledBackgroundColor: AppColors.accentSecondary.withValues(
                  alpha: 0.5,
                ),
                foregroundColor: Colors.white,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(14),
                  side: BorderSide(
                    color: AppColors.accentPrimary.withValues(alpha: 0.7),
                    width: 1.2,
                  ),
                ),
                padding: EdgeInsets.zero,
              ),
              child: AnimatedSwitcher(
                duration: AppMotion.fast,
                child: widget.isLoading
                    ? const SizedBox(
                        key: ValueKey('loading'),
                        width: 22,
                        height: 22,
                        child: CircularProgressIndicator(
                          strokeWidth: 2.5,
                          color: Colors.white,
                        ),
                      )
                    : Row(
                        key: const ValueKey('label'),
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const Icon(Icons.casino_rounded, size: 20),
                          const SizedBox(width: 8),
                          Text(
                            'RANDOM',
                            style: AppTypography.textTheme.labelLarge?.copyWith(
                              color: Colors.white,
                              letterSpacing: 2,
                              fontSize: 14,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ],
                      ),
              ),
            ),
          ),
        );
      },
    );
  }
}
