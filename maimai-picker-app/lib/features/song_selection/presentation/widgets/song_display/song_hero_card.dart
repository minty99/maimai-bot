import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';

import '../../../../../core/theme/app_colors.dart';
import '../../../../../core/theme/app_spacing.dart';
import '../../../../../core/theme/app_typography.dart';
import '../../../data/models/song_model.dart';

/// Hero song card with full-square jacket image (never cropped) and
/// info strip below.  All chart metadata is grouped in the bottom-left
/// overlay area of the jacket.
class SongHeroCard extends StatelessWidget {
  const SongHeroCard({
    super.key,
    required this.song,
    required this.showLevel,
    required this.showUserLevel,
  });

  final SongModel song;
  final bool showLevel;
  final bool showUserLevel;

  @override
  Widget build(BuildContext context) {
    final difficultyColor =
        AppColors.difficultyColors[song.diffCategory.toUpperCase()] ??
        Colors.grey;
    final rankColor = song.rank == null
        ? AppColors.textSecondary
        : AppColors.rankColors[song.rank!.toUpperCase()] ??
              AppColors.textSecondary;
    final showPersonal =
        song.achievementX10000 != null || song.fc != null || song.sync != null;

    return Container(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(20),
        border: Border.all(
          color: difficultyColor.withValues(alpha: 0.65),
          width: 1.4,
        ),
        boxShadow: [
          BoxShadow(
            color: difficultyColor.withValues(alpha: 0.22),
            blurRadius: 24,
            spreadRadius: -8,
          ),
        ],
        color: AppColors.surface,
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(20),
        child: Column(
          children: [
            // ── Jacket area: uncropped square, metadata overlaid ──
            Expanded(
              child: Stack(
                fit: StackFit.expand,
                children: [
                  // Dark background behind jacket
                  const ColoredBox(color: AppColors.background),
                  // Jacket image — BoxFit.contain ensures full visibility
                  if (song.imageUrl.isNotEmpty)
                    CachedNetworkImage(
                      imageUrl: song.imageUrl,
                      fit: BoxFit.contain,
                      placeholder: (_, _) => const Center(
                        child: CircularProgressIndicator(
                          color: AppColors.accentPrimary,
                          strokeWidth: 2.5,
                        ),
                      ),
                      errorWidget: (_, _, _) => const _FallbackCover(),
                    )
                  else
                    const _FallbackCover(),

                  // Gradient overlay at bottom of jacket for readability
                  Positioned(
                    left: 0,
                    right: 0,
                    bottom: 0,
                    height: 80,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          colors: [
                            Colors.transparent,
                            AppColors.surface.withValues(alpha: 0.95),
                          ],
                        ),
                      ),
                    ),
                  ),

                  // Bottom-left: difficulty + chart type + version badges
                  Positioned(
                    left: AppSpacing.md,
                    bottom: AppSpacing.sm,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        _DifficultyBadge(
                          label: song.diffCategory,
                          color: difficultyColor,
                        ),
                        const SizedBox(width: 6),
                        _SmallBadge(label: song.chartType),
                        if (song.version != null) ...[
                          const SizedBox(width: 6),
                          _SmallBadge(label: song.version!),
                        ],
                      ],
                    ),
                  ),
                ],
              ),
            ),

            // ── Info strip below jacket ──
            Padding(
              padding: const EdgeInsets.fromLTRB(
                AppSpacing.md,
                AppSpacing.sm,
                AppSpacing.md,
                AppSpacing.md,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  // Title
                  Text(
                    song.title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: AppTypography.textTheme.titleMedium?.copyWith(
                      color: AppColors.textPrimary,
                      fontWeight: FontWeight.bold,
                    ),
                  ),

                  // Level line: "13+ (13.6 / B+)"
                  if (showLevel) ...[
                    const SizedBox(height: 4),
                    Text(
                      _formatLevelLine(),
                      style: AppTypography.numeric.copyWith(
                        color: AppColors.accentPrimary,
                        fontSize: 14,
                        shadows: [
                          Shadow(
                            color: AppColors.accentPrimary.withValues(
                              alpha: 0.6,
                            ),
                            blurRadius: 6,
                          ),
                        ],
                      ),
                    ),
                  ],

                  // Song count
                  if (song.filteredSongCount != null &&
                      song.levelSongCount != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 2),
                      child: Text(
                        'Picked from ${song.filteredSongCount} / ${song.levelSongCount} songs',
                        style: AppTypography.textTheme.bodySmall?.copyWith(
                          color: AppColors.textMuted,
                          fontSize: 11,
                        ),
                      ),
                    ),

                  // Achievement + rank + FC/Sync badges
                  if (showPersonal && song.achievementX10000 != null) ...[
                    const SizedBox(height: 6),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.baseline,
                      textBaseline: TextBaseline.alphabetic,
                      children: [
                        Text(
                          _formatAchievement(),
                          style: AppTypography.numeric.copyWith(
                            fontSize: 24,
                            color: AppColors.textPrimary,
                          ),
                        ),
                        if (song.rank != null) ...[
                          const SizedBox(width: 8),
                          Text(
                            song.rank!,
                            style: AppTypography.textTheme.titleSmall?.copyWith(
                              color: rankColor,
                            ),
                          ),
                        ],
                        const Spacer(),
                        if (song.fc != null)
                          _GlowTag(label: song.fc!, color: AppColors.badgeGold),
                        if (song.fc != null && song.sync != null)
                          const SizedBox(width: 6),
                        if (song.sync != null)
                          _GlowTag(
                            label: song.sync!,
                            color: AppColors.accentPrimary,
                          ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Build level line: "13+ (13.6 / B+)"
  /// Falls back gracefully when parts are missing.
  String _formatLevelLine() {
    final parts = <String>[];
    final internal = song.internalLevel?.toStringAsFixed(1);
    if (internal != null) parts.add(internal);
    if (showUserLevel && song.userLevel != null && song.userLevel!.isNotEmpty) {
      parts.add(song.userLevel!);
    }

    if (parts.isEmpty) return song.level;
    return '${song.level} (${parts.join(' / ')})';
  }

  String _formatAchievement() {
    if (song.achievementX10000 == null) return '--';
    final percent = song.achievementX10000! / 10000.0;
    return '${percent.toStringAsFixed(4)}%';
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Sub-widgets
// ═══════════════════════════════════════════════════════════════════════════════

class _DifficultyBadge extends StatelessWidget {
  const _DifficultyBadge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(10),
        color: color.withValues(alpha: 0.18),
        border: Border.all(color: color),
        boxShadow: [
          BoxShadow(color: color.withValues(alpha: 0.45), blurRadius: 10),
        ],
      ),
      child: Text(
        label,
        style: AppTypography.textTheme.labelSmall?.copyWith(
          color: color,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _SmallBadge extends StatelessWidget {
  const _SmallBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        color: AppColors.surface.withValues(alpha: 0.85),
        border: Border.all(color: AppColors.textMuted.withValues(alpha: 0.35)),
      ),
      child: Text(
        label,
        style: AppTypography.textTheme.labelSmall?.copyWith(
          color: AppColors.textSecondary,
          fontSize: 11,
        ),
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}

class _GlowTag extends StatelessWidget {
  const _GlowTag({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: color),
        color: color.withValues(alpha: 0.18),
      ),
      child: Text(
        label,
        style: AppTypography.textTheme.labelSmall?.copyWith(
          color: color,
          fontWeight: FontWeight.bold,
        ),
      ),
    );
  }
}

class _FallbackCover extends StatelessWidget {
  const _FallbackCover();

  @override
  Widget build(BuildContext context) {
    return const ColoredBox(
      color: AppColors.surfaceElevated,
      child: Center(
        child: Icon(
          Icons.broken_image_rounded,
          size: 64,
          color: AppColors.textMuted,
        ),
      ),
    );
  }
}
