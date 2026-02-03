import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../settings/presentation/screens/settings_screen.dart';
import '../../../settings/bloc/settings/settings_cubit.dart';
import '../../bloc/hardware_input/hardware_input_cubit.dart';
import '../../bloc/hardware_input/hardware_input_state.dart';
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

class _SongSelectionScreenState extends State<SongSelectionScreen>
    with SingleTickerProviderStateMixin {
  late AnimationController _rangeAnimController;
  late Animation<double> _rangeScaleAnimation;
  LevelRangeState? _previousRangeState;

  @override
  void initState() {
    super.initState();
    // Initialize hardware input cubit
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<HardwareInputCubit>().initialize();
    });

    // Animation for range change feedback
    _rangeAnimController = AnimationController(
      duration: const Duration(milliseconds: 150),
      vsync: this,
    );
    _rangeScaleAnimation = Tween<double>(begin: 1.0, end: 1.08).animate(
      CurvedAnimation(parent: _rangeAnimController, curve: Curves.easeOut),
    );
  }

  @override
  void dispose() {
    _rangeAnimController.dispose();
    super.dispose();
  }

  void _onHardwareInput(BuildContext context, HardwareInputState state) {
    final levelRangeCubit = context.read<LevelRangeCubit>();
    final songCubit = context.read<SongCubit>();
    final rangeState = levelRangeCubit.state;

    if (state is IncrementRangeState) {
      levelRangeCubit.incrementLevel();
    } else if (state is DecrementRangeState) {
      levelRangeCubit.decrementLevel();
    } else if (state is TriggerRandomState) {
      songCubit.fetchRandomSong(
        minLevel: rangeState.start,
        maxLevel: rangeState.end,
      );
    }
  }

  void _triggerRangeAnimation() {
    _rangeAnimController.forward().then((_) => _rangeAnimController.reverse());
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return MultiBlocListener(
      listeners: [
        BlocListener<HardwareInputCubit, HardwareInputState>(
          listener: (context, state) {
            // Handle hardware input states
            if (state is IncrementRangeState ||
                state is DecrementRangeState ||
                state is TriggerRandomState) {
              _onHardwareInput(context, state);
            }
          },
        ),
        BlocListener<LevelRangeCubit, LevelRangeState>(
          listener: (context, state) {
            // Trigger animation when range changes
            if (_previousRangeState != null &&
                (state.start != _previousRangeState!.start ||
                    state.end != _previousRangeState!.end)) {
              _triggerRangeAnimation();
            }
            _previousRangeState = state;
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
              horizontal: 16.0,
              vertical: 8.0,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // ─────────────────────────────────────────────────────────────
                // Controls Row: Level/Gap + Random
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, rangeState) {
                    return BlocBuilder<SongCubit, SongState>(
                      builder: (context, songState) {
                        final isLoading = songState is SongLoading;

                        return Row(
                          crossAxisAlignment: CrossAxisAlignment.center,
                          children: [
                            // Level/Gap Controls
                            Expanded(
                              child: Column(
                                children: [
                                  _CompactAdjustRow(
                                    label: 'LV',
                                    onDecrement: () => context
                                        .read<LevelRangeCubit>()
                                        .decrementLevel(),
                                    onIncrement: () => context
                                        .read<LevelRangeCubit>()
                                        .incrementLevel(),
                                  ),
                                  const SizedBox(height: 4),
                                  _CompactAdjustRow(
                                    label: 'GAP',
                                    onDecrement: () => context
                                        .read<LevelRangeCubit>()
                                        .decrementGap(),
                                    onIncrement: () => context
                                        .read<LevelRangeCubit>()
                                        .incrementGap(),
                                  ),
                                ],
                              ),
                            ),
                            const SizedBox(width: 8),
                            // Random Button
                            SizedBox(
                              width: 72,
                              height: 72,
                              child: FilledButton(
                                onPressed: isLoading
                                    ? null
                                    : () {
                                        context
                                            .read<SongCubit>()
                                            .fetchRandomSong(
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
                                    borderRadius: BorderRadius.circular(14),
                                  ),
                                  padding: EdgeInsets.zero,
                                ),
                                child: isLoading
                                    ? SizedBox(
                                        width: 24,
                                        height: 24,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 3,
                                          color: colorScheme.onPrimary,
                                        ),
                                      )
                                    : Icon(
                                        Icons.casino_rounded,
                                        size: 32,
                                        color: colorScheme.onPrimary,
                                      ),
                              ),
                            ),
                          ],
                        );
                      },
                    );
                  },
                ),
                const SizedBox(height: 8),

                // ─────────────────────────────────────────────────────────────
                // Range Display (horizontal, below buttons)
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, rangeState) {
                    return ScaleTransition(
                      scale: _rangeScaleAnimation,
                      child: _HorizontalRangeDisplay(
                        start: rangeState.start,
                        end: rangeState.end,
                      ),
                    );
                  },
                ),
                const SizedBox(height: 8),

                // ─────────────────────────────────────────────────────────────
                // Song Display Section (fills remaining space)
                // ─────────────────────────────────────────────────────────────
                Expanded(
                  child: Center(
                    child: BlocBuilder<SongCubit, SongState>(
                      builder: (context, state) {
                        return _SongDisplaySection(state: state);
                      },
                    ),
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
// Control Widgets
// ═══════════════════════════════════════════════════════════════════════════════

class _HorizontalRangeDisplay extends StatelessWidget {
  const _HorizontalRangeDisplay({required this.start, required this.end});

  final double start;
  final double end;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      padding: const EdgeInsets.symmetric(vertical: 10, horizontal: 16),
      decoration: BoxDecoration(
        color: colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.primary.withValues(alpha: 0.5),
          width: 2,
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Text(
            'RANGE',
            style: theme.textTheme.labelMedium?.copyWith(
              color: colorScheme.onPrimaryContainer,
              letterSpacing: 1.5,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(width: 16),
          Text(
            start.toStringAsFixed(1),
            style: theme.textTheme.headlineSmall?.copyWith(
              color: colorScheme.onPrimaryContainer,
              fontWeight: FontWeight.bold,
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Text(
              '~',
              style: theme.textTheme.titleLarge?.copyWith(
                color: colorScheme.onPrimaryContainer.withValues(alpha: 0.7),
              ),
            ),
          ),
          Text(
            end.toStringAsFixed(1),
            style: theme.textTheme.headlineSmall?.copyWith(
              color: colorScheme.onPrimaryContainer,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }
}

class _CompactAdjustRow extends StatelessWidget {
  const _CompactAdjustRow({
    required this.label,
    required this.onDecrement,
    required this.onIncrement,
  });

  final String label;
  final VoidCallback onDecrement;
  final VoidCallback onIncrement;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Row(
      children: [
        SizedBox(
          width: 36,
          child: Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: SizedBox(
                  height: 34,
                  child: FilledButton.tonal(
                    onPressed: onDecrement,
                    style: FilledButton.styleFrom(
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                      padding: EdgeInsets.zero,
                    ),
                    child: Icon(
                      Icons.remove,
                      size: 20,
                      color: colorScheme.onSecondaryContainer,
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 6),
              Expanded(
                child: SizedBox(
                  height: 34,
                  child: FilledButton.tonal(
                    onPressed: onIncrement,
                    style: FilledButton.styleFrom(
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                      padding: EdgeInsets.zero,
                    ),
                    child: Icon(
                      Icons.add,
                      size: 20,
                      color: colorScheme.onSecondaryContainer,
                    ),
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
            width: 120,
            height: 120,
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(
                color: colorScheme.outline.withValues(alpha: 0.3),
                width: 2,
              ),
            ),
            child: Icon(
              Icons.music_note_rounded,
              size: 50,
              color: colorScheme.primary.withValues(alpha: 0.5),
            ),
          ),
          const SizedBox(height: 16),
          Text(
            'Press RANDOM to start',
            style: theme.textTheme.titleMedium?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            'or use volume buttons / arrow keys',
            style: theme.textTheme.bodySmall?.copyWith(
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
    final settingsState = context.watch<SettingsCubit>().state;
    final showPersonalData = settingsState.recordCollectorServerUrl
        .trim()
        .isNotEmpty;

    return LayoutBuilder(
      builder: (context, constraints) {
        // Calculate jacket size based on available space
        // Reserve space for title (~60), chart info (~50), version (~40),
        // internal level (~40), achievement (~80), badges (~40), padding (40)
        const estimatedInfoHeight = 310.0;
        final availableHeight = constraints.maxHeight - estimatedInfoHeight;
        final availableWidth = constraints.maxWidth - 32; // Card padding
        final jacketSize =
            (availableHeight > 0 ? availableHeight.clamp(100.0, 200.0) : 150.0)
                .clamp(100.0, availableWidth);

        return SingleChildScrollView(
          child: Card(
            elevation: 4,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(16),
              side: BorderSide(
                color: diffColor.withValues(alpha: 0.5),
                width: 2,
              ),
            ),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // Jacket Image (dynamic size)
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: song.imageUrl.isNotEmpty
                        ? CachedNetworkImage(
                            imageUrl: song.imageUrl,
                            width: jacketSize,
                            height: jacketSize,
                            fit: BoxFit.cover,
                            placeholder: (context, url) => Container(
                              width: jacketSize,
                              height: jacketSize,
                              color: colorScheme.surfaceContainerHighest,
                              child: Center(
                                child: CircularProgressIndicator(
                                  color: colorScheme.primary,
                                ),
                              ),
                            ),
                            errorWidget: (context, url, error) => Container(
                              width: jacketSize,
                              height: jacketSize,
                              color: colorScheme.surfaceContainerHighest,
                              child: Icon(
                                Icons.broken_image_rounded,
                                size: jacketSize * 0.3,
                                color: colorScheme.error,
                              ),
                            ),
                          )
                        : Container(
                            width: jacketSize,
                            height: jacketSize,
                            color: colorScheme.surfaceContainerHighest,
                            child: Icon(
                              Icons.broken_image_rounded,
                              size: jacketSize * 0.3,
                              color: colorScheme.error,
                            ),
                          ),
                  ),
                  const SizedBox(height: 10),

                  // Title
                  Text(
                    song.title,
                    style: theme.textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.bold,
                      color: colorScheme.onSurface,
                    ),
                    textAlign: TextAlign.center,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 8),

                  // Chart Info (type + difficulty + level) + Internal Level
                  Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 6,
                        ),
                        decoration: BoxDecoration(
                          color: diffColor.withValues(alpha: 0.2),
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(color: diffColor, width: 1.5),
                        ),
                        child: Text(
                          '${song.chartType} ${song.diffCategory} ${song.level}',
                          style: theme.textTheme.titleSmall?.copyWith(
                            color: diffColor,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Icon(
                        Icons.bolt_rounded,
                        color: colorScheme.primary,
                        size: 18,
                      ),
                      Text(
                        song.internalLevel?.toStringAsFixed(1) ?? '--',
                        style: theme.textTheme.titleSmall?.copyWith(
                          color: colorScheme.primary,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),

                  if (song.version != null) ...[
                    const SizedBox(height: 6),
                    Text(
                      song.version!,
                      style: theme.textTheme.labelMedium?.copyWith(
                        color: colorScheme.primary,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ],
                  const SizedBox(height: 8),

                  // Achievement + Rank + Badges (compact row)
                  if (showPersonalData && song.achievementX10000 != null) ...[
                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text(
                          _formatAchievement(),
                          style: theme.textTheme.headlineSmall?.copyWith(
                            fontWeight: FontWeight.bold,
                            color: colorScheme.onSurface,
                          ),
                        ),
                        if (song.rank != null) ...[
                          const SizedBox(width: 8),
                          Text(
                            song.rank!,
                            style: theme.textTheme.titleLarge?.copyWith(
                              color: _getRankColor(song.rank!),
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ],
                      ],
                    ),
                    const SizedBox(height: 6),
                  ],

                  // FC/Sync Badges
                  if (showPersonalData &&
                      (song.fc != null || song.sync != null))
                    Wrap(
                      spacing: 8,
                      children: [
                        if (song.fc != null)
                          _Badge(
                            label: song.fc!,
                            color: const Color(0xFFFFD700),
                          ),
                        if (song.sync != null)
                          _Badge(
                            label: song.sync!,
                            color: const Color(0xFF00BFFF),
                          ),
                      ],
                    ),
                ],
              ),
            ),
          ),
        );
      },
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
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color, width: 1.5),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontWeight: FontWeight.bold,
          fontSize: 13,
        ),
      ),
    );
  }
}
