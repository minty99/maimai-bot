import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../settings/presentation/screens/settings_screen.dart';
import '../../bloc/hardware_input/hardware_input_cubit.dart';
import '../../bloc/hardware_input/hardware_input_event.dart';
import '../../bloc/level_range/level_range_cubit.dart';
import '../../bloc/level_range/level_range_state.dart';
import '../../bloc/song/song_cubit.dart';
import '../../bloc/song/song_state.dart';
import '../../data/models/song_model.dart';

/// Main screen for random song selection.
///
/// Features:
/// - Level range display and controls (large, glove-friendly buttons)
/// - Gap adjustment options
/// - Random song button with loading state
/// - Song display with jacket art, metadata, and score info
/// - Hardware input integration (volume buttons / arrow keys)
class SongSelectionScreen extends StatefulWidget {
  const SongSelectionScreen({super.key});

  static const String routeName = '/';

  @override
  State<SongSelectionScreen> createState() => _SongSelectionScreenState();
}

class _SongSelectionScreenState extends State<SongSelectionScreen> {
  @override
  void initState() {
    super.initState();
    // Initialize hardware input cubit
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<HardwareInputCubit>().initialize();
    });
  }

  void _onHardwareInput(BuildContext context, HardwareInputEvent event) {
    final levelRangeCubit = context.read<LevelRangeCubit>();
    final songCubit = context.read<SongCubit>();
    final rangeState = levelRangeCubit.state;

    if (event is IncrementRange) {
      levelRangeCubit.incrementStart();
    } else if (event is DecrementRange) {
      levelRangeCubit.decrementStart();
    } else if (event is TriggerRandom) {
      songCubit.fetchRandomSong(
        minLevel: rangeState.start,
        maxLevel: rangeState.end,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return MultiBlocListener(
      listeners: [
        BlocListener<HardwareInputCubit, dynamic>(
          listener: (context, state) {
            // Handle hardware input events
            if (state is IncrementRange ||
                state is DecrementRange ||
                state is TriggerRandom) {
              _onHardwareInput(context, state as HardwareInputEvent);
            }
          },
        ),
      ],
      child: Scaffold(
        appBar: AppBar(
          title: const Text('maimai Randomizer'),
          actions: [
            IconButton(
              icon: const Icon(Icons.settings),
              iconSize: 28,
              onPressed: () {
                Navigator.pushNamed(context, SettingsScreen.routeName);
              },
            ),
          ],
        ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: 24.0,
              vertical: 16.0,
            ),
            child: Column(
              children: [
                // ─────────────────────────────────────────────────────────────
                // Range Display Section
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, state) {
                    return _RangeDisplaySection(
                      start: state.start,
                      end: state.end,
                      gap: state.gap,
                    );
                  },
                ),
                const SizedBox(height: 24),

                // ─────────────────────────────────────────────────────────────
                // Range Controls Section (large buttons)
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, state) {
                    return _RangeControlsSection(
                      onDecrement: () =>
                          context.read<LevelRangeCubit>().decrementStart(),
                      onIncrement: () =>
                          context.read<LevelRangeCubit>().incrementStart(),
                    );
                  },
                ),
                const SizedBox(height: 16),

                // ─────────────────────────────────────────────────────────────
                // Gap Adjustment Section
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, state) {
                    return _GapAdjustmentSection(
                      currentGap: state.gap,
                      onGapChanged: (gap) =>
                          context.read<LevelRangeCubit>().adjustGap(gap),
                    );
                  },
                ),
                const SizedBox(height: 24),

                // ─────────────────────────────────────────────────────────────
                // RANDOM Button
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, rangeState) {
                    return BlocBuilder<SongCubit, SongState>(
                      builder: (context, songState) {
                        final isLoading = songState is SongLoading;

                        return SizedBox(
                          width: double.infinity,
                          height: 100,
                          child: FilledButton(
                            onPressed: isLoading
                                ? null
                                : () {
                                    context.read<SongCubit>().fetchRandomSong(
                                      minLevel: rangeState.start,
                                      maxLevel: rangeState.end,
                                    );
                                  },
                            style: FilledButton.styleFrom(
                              backgroundColor: colorScheme.primary,
                              foregroundColor: colorScheme.onPrimary,
                              disabledBackgroundColor: colorScheme.primary
                                  .withValues(alpha: 0.5),
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(20),
                              ),
                            ),
                            child: isLoading
                                ? SizedBox(
                                    width: 32,
                                    height: 32,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 3,
                                      color: colorScheme.onPrimary,
                                    ),
                                  )
                                : Text(
                                    'RANDOM',
                                    style: theme.textTheme.headlineMedium
                                        ?.copyWith(
                                          color: colorScheme.onPrimary,
                                          fontWeight: FontWeight.bold,
                                          letterSpacing: 2,
                                        ),
                                  ),
                          ),
                        );
                      },
                    );
                  },
                ),
                const SizedBox(height: 24),

                // ─────────────────────────────────────────────────────────────
                // Song Display Section
                // ─────────────────────────────────────────────────────────────
                Expanded(
                  child: BlocBuilder<SongCubit, SongState>(
                    builder: (context, state) {
                      return _SongDisplaySection(state: state);
                    },
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Range Display Section
// ═══════════════════════════════════════════════════════════════════════════════

class _RangeDisplaySection extends StatelessWidget {
  const _RangeDisplaySection({
    required this.start,
    required this.end,
    required this.gap,
  });

  final double start;
  final double end;
  final double gap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      padding: const EdgeInsets.symmetric(vertical: 20, horizontal: 32),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(20),
        border: Border.all(
          color: colorScheme.primary.withValues(alpha: 0.3),
          width: 2,
        ),
      ),
      child: Column(
        children: [
          Text(
            'LEVEL RANGE',
            style: theme.textTheme.labelLarge?.copyWith(
              color: colorScheme.primary,
              letterSpacing: 2,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            '${start.toStringAsFixed(1)} - ${end.toStringAsFixed(1)}',
            style: theme.textTheme.displayMedium?.copyWith(
              color: colorScheme.onSurface,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Range Controls Section
// ═══════════════════════════════════════════════════════════════════════════════

class _RangeControlsSection extends StatelessWidget {
  const _RangeControlsSection({
    required this.onDecrement,
    required this.onIncrement,
  });

  final VoidCallback onDecrement;
  final VoidCallback onIncrement;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        // Decrement button (80x80)
        SizedBox(
          width: 80,
          height: 80,
          child: FilledButton.tonal(
            onPressed: onDecrement,
            style: FilledButton.styleFrom(
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            child: Icon(
              Icons.remove,
              size: 40,
              color: colorScheme.onSecondaryContainer,
            ),
          ),
        ),
        const SizedBox(width: 32),
        // Increment button (80x80)
        SizedBox(
          width: 80,
          height: 80,
          child: FilledButton.tonal(
            onPressed: onIncrement,
            style: FilledButton.styleFrom(
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            child: Icon(
              Icons.add,
              size: 40,
              color: colorScheme.onSecondaryContainer,
            ),
          ),
        ),
      ],
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Gap Adjustment Section
// ═══════════════════════════════════════════════════════════════════════════════

class _GapAdjustmentSection extends StatelessWidget {
  const _GapAdjustmentSection({
    required this.currentGap,
    required this.onGapChanged,
  });

  final double currentGap;
  final void Function(double) onGapChanged;

  static const List<double> gapOptions = [0.05, 0.1, 0.2, 0.5];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Column(
      children: [
        Text(
          'Gap: ${currentGap.toStringAsFixed(2)}',
          style: theme.textTheme.bodyLarge?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          children: gapOptions.map((gap) {
            final isSelected = (currentGap - gap).abs() < 0.001;

            return SizedBox(
              width: 64,
              height: 48,
              child: isSelected
                  ? FilledButton(
                      onPressed: () => onGapChanged(gap),
                      style: FilledButton.styleFrom(
                        padding: EdgeInsets.zero,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12),
                        ),
                      ),
                      child: Text(
                        gap.toString(),
                        style: theme.textTheme.labelLarge?.copyWith(
                          color: colorScheme.onPrimary,
                        ),
                      ),
                    )
                  : OutlinedButton(
                      onPressed: () => onGapChanged(gap),
                      style: OutlinedButton.styleFrom(
                        padding: EdgeInsets.zero,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(12),
                        ),
                      ),
                      child: Text(
                        gap.toString(),
                        style: theme.textTheme.labelLarge?.copyWith(
                          color: colorScheme.onSurface,
                        ),
                      ),
                    ),
            );
          }).toList(),
        ),
      ],
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Song Display Section
// ═══════════════════════════════════════════════════════════════════════════════

class _SongDisplaySection extends StatelessWidget {
  const _SongDisplaySection({required this.state});

  final SongState state;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return switch (state) {
      SongInitial() => _InitialState(theme: theme, colorScheme: colorScheme),
      SongLoading() => const Center(
        child: CircularProgressIndicator(strokeWidth: 4),
      ),
      SongNotFound() => _NotFoundState(theme: theme, colorScheme: colorScheme),
      SongError(:final message) => _ErrorState(
        message: message,
        theme: theme,
        colorScheme: colorScheme,
      ),
      SongLoaded(:final song) => _LoadedState(
        song: song,
        theme: theme,
        colorScheme: colorScheme,
      ),
    };
  }
}

class _InitialState extends StatelessWidget {
  const _InitialState({required this.theme, required this.colorScheme});

  final ThemeData theme;
  final ColorScheme colorScheme;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 180,
            height: 180,
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(24),
              border: Border.all(
                color: colorScheme.outline.withValues(alpha: 0.3),
                width: 2,
              ),
            ),
            child: Icon(
              Icons.music_note_rounded,
              size: 80,
              color: colorScheme.primary.withValues(alpha: 0.5),
            ),
          ),
          const SizedBox(height: 24),
          Text(
            'Press RANDOM to start',
            style: theme.textTheme.titleLarge?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'or use volume buttons / arrow keys',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
            ),
          ),
        ],
      ),
    );
  }
}

class _NotFoundState extends StatelessWidget {
  const _NotFoundState({required this.theme, required this.colorScheme});

  final ThemeData theme;
  final ColorScheme colorScheme;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.search_off_rounded,
            size: 80,
            color: colorScheme.error.withValues(alpha: 0.7),
          ),
          const SizedBox(height: 16),
          Text(
            'No songs in this range',
            style: theme.textTheme.titleLarge?.copyWith(
              color: colorScheme.error,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Try adjusting the level range',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }
}

class _ErrorState extends StatelessWidget {
  const _ErrorState({
    required this.message,
    required this.theme,
    required this.colorScheme,
  });

  final String message;
  final ThemeData theme;
  final ColorScheme colorScheme;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.error_outline_rounded, size: 80, color: colorScheme.error),
          const SizedBox(height: 16),
          Text(
            'Error',
            style: theme.textTheme.titleLarge?.copyWith(
              color: colorScheme.error,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            message,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _LoadedState extends StatelessWidget {
  const _LoadedState({
    required this.song,
    required this.theme,
    required this.colorScheme,
  });

  final SongModel song;
  final ThemeData theme;
  final ColorScheme colorScheme;

  Color _getDifficultyColor() {
    return switch (song.diffCategory.toUpperCase()) {
      'BASIC' => const Color(0xFF69C36D),
      'ADVANCED' => const Color(0xFFF4C430),
      'EXPERT' => const Color(0xFFFF6B8A),
      'MASTER' => const Color(0xFF9B59B6),
      'RE:MASTER' => Colors.white,
      _ => Colors.grey,
    };
  }

  String _formatAchievement() {
    if (song.achievementX10000 == null) return '--';
    final percent = song.achievementX10000! / 10000.0;
    return '${percent.toStringAsFixed(4)}%';
  }

  @override
  Widget build(BuildContext context) {
    final diffColor = _getDifficultyColor();

    return SingleChildScrollView(
      child: Card(
        elevation: 8,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(24),
          side: BorderSide(color: diffColor.withValues(alpha: 0.5), width: 2),
        ),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Jacket Image
              ClipRRect(
                borderRadius: BorderRadius.circular(16),
                child: CachedNetworkImage(
                  imageUrl: song.imageUrl,
                  width: 280,
                  height: 280,
                  fit: BoxFit.cover,
                  placeholder: (context, url) => Container(
                    width: 280,
                    height: 280,
                    color: colorScheme.surfaceContainerHighest,
                    child: Center(
                      child: CircularProgressIndicator(
                        color: colorScheme.primary,
                      ),
                    ),
                  ),
                  errorWidget: (context, url, error) => Container(
                    width: 280,
                    height: 280,
                    color: colorScheme.surfaceContainerHighest,
                    child: Icon(
                      Icons.broken_image_rounded,
                      size: 80,
                      color: colorScheme.error,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 16),

              // Title
              Text(
                song.title,
                style: theme.textTheme.headlineMedium?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: colorScheme.onSurface,
                ),
                textAlign: TextAlign.center,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 12),

              // Chart Info (type + difficulty + level)
              Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 8,
                ),
                decoration: BoxDecoration(
                  color: diffColor.withValues(alpha: 0.2),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: diffColor, width: 1.5),
                ),
                child: Text(
                  '${song.chartType}  ${song.diffCategory}  ${song.level}',
                  style: theme.textTheme.titleLarge?.copyWith(
                    color: diffColor,
                    fontWeight: FontWeight.w600,
                    letterSpacing: 1,
                  ),
                ),
              ),
              const SizedBox(height: 12),

              // Internal Level
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    Icons.bolt_rounded,
                    color: colorScheme.primary,
                    size: 24,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    song.internalLevel.toStringAsFixed(1),
                    style: theme.textTheme.titleLarge?.copyWith(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // Achievement + Rank
              if (song.achievementX10000 != null) ...[
                Text(
                  _formatAchievement(),
                  style: theme.textTheme.displayMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: colorScheme.onSurface,
                  ),
                ),
                if (song.rank != null)
                  Text(
                    song.rank!,
                    style: theme.textTheme.headlineMedium?.copyWith(
                      color: _getRankColor(song.rank!),
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                const SizedBox(height: 12),
              ],

              // FC/Sync Badges
              if (song.fc != null || song.sync != null)
                Wrap(
                  spacing: 12,
                  children: [
                    if (song.fc != null)
                      _Badge(label: song.fc!, color: const Color(0xFFFFD700)),
                    if (song.sync != null)
                      _Badge(label: song.sync!, color: const Color(0xFF00BFFF)),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }

  Color _getRankColor(String rank) {
    return switch (rank.toUpperCase()) {
      'SSS+' || 'SSS' => const Color(0xFFFFD700), // Gold
      'SS+' || 'SS' => const Color(0xFFFFA500), // Orange
      'S+' || 'S' => const Color(0xFFFF6B8A), // Pink
      'AAA' || 'AA' || 'A' => const Color(0xFF69C36D), // Green
      _ => Colors.grey,
    };
  }
}

class _Badge extends StatelessWidget {
  const _Badge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: color, width: 2),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontWeight: FontWeight.bold,
          fontSize: 16,
        ),
      ),
    );
  }
}
