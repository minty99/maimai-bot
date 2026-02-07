import 'package:cached_network_image/cached_network_image.dart';
import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../../core/constants/app_constants.dart';
import '../../../settings/bloc/settings/settings_cubit.dart';
import '../../../settings/bloc/settings/settings_state.dart';
import '../../../settings/presentation/screens/settings_screen.dart';
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
  List<_VersionOption> _versionOptions = const [];
  bool _isLoadingVersions = false;

  @override
  void initState() {
    super.initState();
    // Initialize hardware input cubit
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<HardwareInputCubit>().initialize();
      _loadVersionOptions();
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

  Future<void> _loadVersionOptions() async {
    final state = context.read<SettingsCubit>().state;
    var baseUrl = state.songInfoServerUrl.trim();
    if (baseUrl.endsWith('/')) {
      baseUrl = baseUrl.substring(0, baseUrl.length - 1);
    }
    if (baseUrl.isEmpty) return;

    setState(() => _isLoadingVersions = true);

    final dio = Dio(
      BaseOptions(
        connectTimeout: const Duration(seconds: 5),
        receiveTimeout: const Duration(seconds: 5),
      ),
    );

    try {
      final response = await dio.get<Map<String, dynamic>>(
        '$baseUrl/api/songs/versions',
      );
      final rawVersions = response.data?['versions'] as List<dynamic>? ?? [];
      final parsed = rawVersions
          .map((raw) => _VersionOption.fromJson(raw as Map<String, dynamic>))
          .where((v) => v.versionIndex >= 0)
          .toList()
        ..sort((a, b) => a.versionIndex.compareTo(b.versionIndex));
      if (mounted) setState(() => _versionOptions = parsed);
    } catch (_) {
      // Silently fail - versions will be empty
    } finally {
      if (mounted) setState(() => _isLoadingVersions = false);
    }
  }

  Future<void> _toggleChartType(String chartType, bool selected) async {
    final cubit = context.read<SettingsCubit>();
    final next = {...cubit.state.enabledChartTypes};
    if (selected) {
      next.add(chartType);
    } else {
      if (next.length == 1) return; // At least one must remain
      next.remove(chartType);
    }
    await cubit.updateEnabledChartTypes(next);
  }

  Future<void> _toggleDifficulty(int index, bool selected) async {
    final cubit = context.read<SettingsCubit>();
    final next = {...cubit.state.enabledDifficultyIndices};
    if (selected) {
      next.add(index);
    } else {
      if (next.length == 1) return; // At least one must remain
      next.remove(index);
    }
    await cubit.updateEnabledDifficulties(next);
  }

  Future<void> _toggleVersion(int versionIndex, bool selected) async {
    final cubit = context.read<SettingsCubit>();
    final state = cubit.state;
    final allIndices = _versionOptions.map((v) => v.versionIndex).toSet();
    if (allIndices.isEmpty) return;

    final current = state.includeVersionIndices;
    if (selected) {
      if (current == null) return; // Already all selected
      final next = {...current, versionIndex};
      if (next.length == allIndices.length) {
        await cubit.updateIncludeVersionIndices(null);
      } else {
        await cubit.updateIncludeVersionIndices(next);
      }
    } else {
      final base = current == null ? {...allIndices} : {...current};
      base.remove(versionIndex);
      await cubit.updateIncludeVersionIndices(base);
    }
  }

  String _buildChartTypeLabel(SettingsState state) {
    final enabled = state.enabledChartTypes;
    if (enabled.length == AppConstants.defaultEnabledChartTypes.length) {
      return 'ALL';
    }
    return enabled.join('/');
  }

  String _buildDifficultyLabel(SettingsState state) {
    final enabled = state.enabledDifficultyIndices;
    if (enabled.length == AppConstants.defaultEnabledDifficultyIndices.length) {
      return 'ALL';
    }
    // Show abbreviated difficulty names
    final names = enabled.map((i) {
      return switch (i) {
        0 => 'B',
        1 => 'A',
        2 => 'E',
        3 => 'M',
        4 => 'R',
        _ => '?',
      };
    }).toList()
      ..sort();
    return names.join('');
  }

  String _buildVersionLabel(SettingsState state) {
    final included = state.includeVersionIndices;
    if (included == null) return 'ALL';
    if (included.isEmpty) return 'NONE';
    if (_versionOptions.isNotEmpty &&
        included.length == _versionOptions.length) {
      return 'ALL';
    }
    return '${included.length}';
  }

  void _showChartTypePopup(BuildContext context, SettingsState state) {
    final colorScheme = Theme.of(context).colorScheme;
    showDialog(
      context: context,
      builder: (ctx) => BlocBuilder<SettingsCubit, SettingsState>(
        builder: (context, currentState) {
          return AlertDialog(
            title: const Text('Chart Type'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: AppConstants.defaultEnabledChartTypes.map((type) {
                final selected = currentState.enabledChartTypes.contains(type);
                return CheckboxListTile(
                  dense: true,
                  title: Text(type),
                  value: selected,
                  onChanged: (checked) {
                    _toggleChartType(type, checked ?? false);
                  },
                );
              }).toList(),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(ctx),
                child:
                    Text('CLOSE', style: TextStyle(color: colorScheme.primary)),
              ),
            ],
          );
        },
      ),
    );
  }

  void _showDifficultyPopup(BuildContext context, SettingsState state) {
    final colorScheme = Theme.of(context).colorScheme;
    showDialog(
      context: context,
      builder: (ctx) => BlocBuilder<SettingsCubit, SettingsState>(
        builder: (context, currentState) {
          return AlertDialog(
            title: const Text('Difficulty'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children:
                  AppConstants.difficultyLabelsByIndex.entries.map((entry) {
                final selected =
                    currentState.enabledDifficultyIndices.contains(entry.key);
                return CheckboxListTile(
                  dense: true,
                  title: Text(entry.value),
                  value: selected,
                  onChanged: (checked) {
                    _toggleDifficulty(entry.key, checked ?? false);
                  },
                );
              }).toList(),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(ctx),
                child:
                    Text('CLOSE', style: TextStyle(color: colorScheme.primary)),
              ),
            ],
          );
        },
      ),
    );
  }

  void _showVersionPopup(BuildContext context, SettingsState state) {
    final colorScheme = Theme.of(context).colorScheme;
    final theme = Theme.of(context);

    if (_isLoadingVersions) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Loading versions...')),
      );
      return;
    }

    if (_versionOptions.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No version data available')),
      );
      return;
    }

    showDialog(
      context: context,
      builder: (ctx) => BlocBuilder<SettingsCubit, SettingsState>(
        builder: (context, currentState) {
          return AlertDialog(
            title: Row(
              children: [
                const Expanded(child: Text('Versions')),
                TextButton(
                  onPressed: () {
                    context
                        .read<SettingsCubit>()
                        .updateIncludeVersionIndices(null);
                  },
                  child: const Text('ALL'),
                ),
                TextButton(
                  onPressed: () {
                    context
                        .read<SettingsCubit>()
                        .updateIncludeVersionIndices(<int>{});
                  },
                  child: const Text('NONE'),
                ),
              ],
            ),
            content: SizedBox(
              width: double.maxFinite,
              height: 300,
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: _versionOptions.length,
                itemBuilder: (_, index) {
                  final version = _versionOptions[index];
                  final selected =
                      currentState.includeVersionIndices == null ||
                          currentState.includeVersionIndices!
                              .contains(version.versionIndex);
                  return CheckboxListTile(
                    dense: true,
                    value: selected,
                    onChanged: (checked) {
                      _toggleVersion(version.versionIndex, checked ?? false);
                    },
                    title: Text(version.versionName),
                    subtitle: Text(
                      '${version.songCount} songs',
                      style: theme.textTheme.bodySmall,
                    ),
                  );
                },
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(ctx),
                child:
                    Text('CLOSE', style: TextStyle(color: colorScheme.primary)),
              ),
            ],
          );
        },
      ),
    );
  }

  void _onHardwareInput(BuildContext context, HardwareInputState state) {
    final levelRangeCubit = context.read<LevelRangeCubit>();
    final songCubit = context.read<SongCubit>();
    final rangeState = levelRangeCubit.state;
    final settingsState = context.read<SettingsCubit>().state;

    if (state is IncrementRangeState) {
      levelRangeCubit.incrementLevel();
    } else if (state is DecrementRangeState) {
      levelRangeCubit.decrementLevel();
    } else if (state is TriggerRandomState) {
      songCubit.fetchRandomSong(
        minLevel: rangeState.start,
        maxLevel: rangeState.end,
        chartTypes: settingsState.enabledChartTypes,
        difficultyIndices: settingsState.enabledDifficultyIndices,
        includeVersionIndices: settingsState.includeVersionIndices,
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
          title: const Text('maimai picker'),
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
                                              chartTypes: context
                                                  .read<SettingsCubit>()
                                                  .state
                                                  .enabledChartTypes,
                                              difficultyIndices: context
                                                  .read<SettingsCubit>()
                                                  .state
                                                  .enabledDifficultyIndices,
                                              includeVersionIndices: context
                                                  .read<SettingsCubit>()
                                                  .state
                                                  .includeVersionIndices,
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
                const SizedBox(height: 6),

                // ─────────────────────────────────────────────────────────────
                // Filter Buttons Row
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<SettingsCubit, SettingsState>(
                  builder: (context, settingsState) {
                    return Row(
                      children: [
                        Expanded(
                          child: _FilterButton(
                            icon: Icons.library_music_outlined,
                            label: _buildChartTypeLabel(settingsState),
                            onTap: () => _showChartTypePopup(
                              context,
                              settingsState,
                            ),
                          ),
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: _FilterButton(
                            icon: Icons.star_outline,
                            label: _buildDifficultyLabel(settingsState),
                            onTap: () => _showDifficultyPopup(
                              context,
                              settingsState,
                            ),
                          ),
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: _FilterButton(
                            icon: Icons.history,
                            label: _buildVersionLabel(settingsState),
                            onTap: () => _showVersionPopup(
                              context,
                              settingsState,
                            ),
                          ),
                        ),
                      ],
                    );
                  },
                ),
                const SizedBox(height: 6),

                // ─────────────────────────────────────────────────────────────
                // Range Display
                // ─────────────────────────────────────────────────────────────
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, rangeState) {
                    return ScaleTransition(
                      scale: _rangeScaleAnimation,
                      child: _CompactRangeDisplay(
                        start: rangeState.start,
                        end: rangeState.end,
                      ),
                    );
                  },
                ),
                const SizedBox(height: 6),

                // ─────────────────────────────────────────────────────────────
                // Song Display Section (fills remaining space)
                // ─────────────────────────────────────────────────────────────
                Expanded(
                  child: Center(
                    child: BlocBuilder<SettingsCubit, SettingsState>(
                      buildWhen: (previous, current) {
                        return previous.showLevel != current.showLevel ||
                            previous.showUserLevel != current.showUserLevel;
                      },
                      builder: (context, settingsState) {
                        return BlocBuilder<SongCubit, SongState>(
                          builder: (context, state) {
                            return _SongDisplaySection(
                              state: state,
                              showLevel: settingsState.showLevel,
                              showUserLevel: settingsState.showUserLevel,
                            );
                          },
                        );
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

class _CompactRangeDisplay extends StatelessWidget {
  const _CompactRangeDisplay({required this.start, required this.end});

  final double start;
  final double end;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 10),
      decoration: BoxDecoration(
        color: colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.primary.withValues(alpha: 0.5),
          width: 1.5,
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Text(
            start.toStringAsFixed(1),
            style: theme.textTheme.titleMedium?.copyWith(
              color: colorScheme.onPrimaryContainer,
              fontWeight: FontWeight.bold,
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6),
            child: Text(
              '~',
              style: theme.textTheme.titleMedium?.copyWith(
                color: colorScheme.onPrimaryContainer.withValues(alpha: 0.7),
              ),
            ),
          ),
          Text(
            end.toStringAsFixed(1),
            style: theme.textTheme.titleMedium?.copyWith(
              color: colorScheme.onPrimaryContainer,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }
}

class _FilterButton extends StatelessWidget {
  const _FilterButton({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Material(
      color: colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(8),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                icon,
                size: 16,
                color: colorScheme.onSecondaryContainer,
              ),
              const SizedBox(width: 4),
              Text(
                label,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: colorScheme.onSecondaryContainer,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ),
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
  const _SongDisplaySection({
    required this.state,
    required this.showLevel,
    required this.showUserLevel,
  });

  final SongState state;
  final bool showLevel;
  final bool showUserLevel;

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
        showLevel: showLevel,
        showUserLevel: showUserLevel,
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
    required this.showLevel,
    required this.showUserLevel,
  });

  final SongModel song;
  final ThemeData theme;
  final ColorScheme colorScheme;
  final bool showLevel;
  final bool showUserLevel;

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

  String _formatInternal() {
    final internal = song.internalLevel?.toStringAsFixed(1) ?? '--';
    final ul = song.userLevel;
    if (showUserLevel && ul != null && ul.isNotEmpty) {
      return '$internal ($ul)';
    }
    return internal;
  }

  String _formatAchievement() {
    if (song.achievementX10000 == null) return '--';
    final percent = song.achievementX10000! / 10000.0;
    return '${percent.toStringAsFixed(4)}%';
  }

  @override
  Widget build(BuildContext context) {
    final diffColor = _getDifficultyColor();
    // Show personal data if any achievement data is present
    // (record-collector-server was reachable and returned data)
    final showPersonalData =
        song.achievementX10000 != null || song.fc != null || song.sync != null;

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

                  // Chart/difficulty/internal row is fully hidden when level display is off.
                  if (showLevel)
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
                          _formatInternal(),
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
                  if (song.filteredSongCount != null &&
                      song.levelSongCount != null) ...[
                    const SizedBox(height: 4),
                    Text(
                      'Picked from ${song.filteredSongCount} / ${song.levelSongCount} songs (filtered / level)',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                      textAlign: TextAlign.center,
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

class _VersionOption {
  const _VersionOption({
    required this.versionIndex,
    required this.versionName,
    required this.songCount,
  });

  final int versionIndex;
  final String versionName;
  final int songCount;

  factory _VersionOption.fromJson(Map<String, dynamic> json) {
    return _VersionOption(
      versionIndex: (json['version_index'] as num?)?.toInt() ?? -1,
      versionName: json['version_name'] as String? ?? 'Unknown',
      songCount: (json['song_count'] as num?)?.toInt() ?? 0,
    );
  }
}
