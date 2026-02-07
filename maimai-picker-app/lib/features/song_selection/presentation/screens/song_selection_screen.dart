import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../../core/constants/app_constants.dart';
import '../../../../core/theme/app_colors.dart';
import '../../../../core/theme/app_motion.dart';
import '../../../../core/theme/app_spacing.dart';
import '../../../settings/bloc/settings/settings_cubit.dart';
import '../../../settings/bloc/settings/settings_state.dart';
import '../../../settings/presentation/screens/settings_screen.dart';
import '../../bloc/hardware_input/hardware_input_cubit.dart';
import '../../bloc/hardware_input/hardware_input_state.dart';
import '../../bloc/level_range/level_range_cubit.dart';
import '../../bloc/level_range/level_range_state.dart';
import '../../bloc/song/song_cubit.dart';
import '../../bloc/song/song_state.dart';
import '../../data/models/version_option.dart';
import '../../data/repositories/version_repository.dart';
import '../widgets/controls/level_gap_controls.dart';
import '../widgets/controls/random_button.dart';
import '../widgets/filters/filter_bottom_sheet.dart';
import '../widgets/filters/filter_chip_bar.dart';
import '../widgets/song_display/song_display_section.dart';

class SongSelectionScreen extends StatefulWidget {
  const SongSelectionScreen({super.key});

  static const String routeName = '/';

  @override
  State<SongSelectionScreen> createState() => _SongSelectionScreenState();
}

class _SongSelectionScreenState extends State<SongSelectionScreen>
    with SingleTickerProviderStateMixin {
  late final AnimationController _rangeAnimController;
  late final Animation<double> _rangeScaleAnimation;

  final VersionRepository _versionRepository = VersionRepositoryImpl();
  LevelRangeState? _previousRangeState;
  List<VersionOption> _versionOptions = const [];
  bool _isLoadingVersions = false;

  @override
  void initState() {
    super.initState();
    _rangeAnimController = AnimationController(
      duration: AppMotion.fast,
      vsync: this,
    );
    _rangeScaleAnimation = Tween<double>(begin: 1, end: 1.08).animate(
      CurvedAnimation(
        parent: _rangeAnimController,
        curve: AppMotion.emphasized,
      ),
    );

    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<HardwareInputCubit>().initialize();
      _loadVersionOptions(
        context.read<SettingsCubit>().state.songInfoServerUrl,
      );
    });
  }

  @override
  void dispose() {
    _rangeAnimController.dispose();
    super.dispose();
  }

  Future<void> _loadVersionOptions(String baseUrl) async {
    setState(() => _isLoadingVersions = true);
    final versions = await _versionRepository.fetchVersionOptions(
      baseUrl: baseUrl,
    );
    if (!mounted) {
      return;
    }
    setState(() {
      _versionOptions = versions;
      _isLoadingVersions = false;
    });
  }

  Future<void> _toggleChartType(String chartType, bool selected) async {
    final cubit = context.read<SettingsCubit>();
    final next = {...cubit.state.enabledChartTypes};
    if (selected) {
      next.add(chartType);
    } else {
      if (next.length == 1) {
        return;
      }
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
      if (next.length == 1) {
        return;
      }
      next.remove(index);
    }
    await cubit.updateEnabledDifficulties(next);
  }

  Future<void> _toggleVersion(int versionIndex, bool selected) async {
    final cubit = context.read<SettingsCubit>();
    final state = cubit.state;
    final allIndices = _versionOptions
        .map((version) => version.versionIndex)
        .toSet();
    if (allIndices.isEmpty) {
      return;
    }

    final current = state.includeVersionIndices;
    if (selected) {
      if (current == null) {
        return;
      }
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
    if (state.enabledChartTypes.length ==
        AppConstants.defaultEnabledChartTypes.length) {
      return 'TYPE ALL';
    }
    final labels = state.enabledChartTypes.toList()..sort();
    return labels.join('/');
  }

  String _buildDifficultyLabel(SettingsState state) {
    if (state.enabledDifficultyIndices.length ==
        AppConstants.defaultEnabledDifficultyIndices.length) {
      return 'DIFF ALL';
    }
    final names = state.enabledDifficultyIndices.map((index) {
      return switch (index) {
        0 => 'B',
        1 => 'A',
        2 => 'E',
        3 => 'M',
        4 => 'R',
        _ => '?',
      };
    }).toList()..sort();
    return 'DIFF ${names.join()}';
  }

  String _buildVersionLabel(SettingsState state) {
    final included = state.includeVersionIndices;
    if (included == null) {
      return 'VER ALL';
    }
    if (included.isEmpty) {
      return 'VER NONE';
    }
    if (_versionOptions.isNotEmpty &&
        included.length == _versionOptions.length) {
      return 'VER ALL';
    }
    return 'VER ${included.length}';
  }

  Future<void> _fetchRandomSong() async {
    final rangeState = context.read<LevelRangeCubit>().state;
    final settingsState = context.read<SettingsCubit>().state;
    await context.read<SongCubit>().fetchRandomSong(
      minLevel: rangeState.start,
      maxLevel: rangeState.end,
      chartTypes: settingsState.enabledChartTypes,
      difficultyIndices: settingsState.enabledDifficultyIndices,
      includeVersionIndices: settingsState.includeVersionIndices,
    );
  }

  void _onHardwareInput(HardwareInputState state) {
    final levelRangeCubit = context.read<LevelRangeCubit>();
    if (state is IncrementRangeState) {
      levelRangeCubit.incrementLevel();
    } else if (state is DecrementRangeState) {
      levelRangeCubit.decrementLevel();
    } else if (state is TriggerRandomState) {
      _fetchRandomSong();
    }
  }

  void _showChartTypeSheet(SettingsState state) {
    showFilterBottomSheet<String>(
      context: context,
      title: 'Chart Type',
      options: AppConstants.defaultEnabledChartTypes
          .map((type) => FilterOption(value: type, label: type))
          .toList(),
      isSelected: (value) =>
          context.read<SettingsCubit>().state.enabledChartTypes.contains(value),
      onSelectAll: (_) => context.read<SettingsCubit>().updateEnabledChartTypes(
        AppConstants.defaultEnabledChartTypes.toSet(),
      ),
      onToggle: _toggleChartType,
    );
  }

  void _showDifficultySheet(SettingsState state) {
    showFilterBottomSheet<int>(
      context: context,
      title: 'Difficulty',
      options: AppConstants.difficultyLabelsByIndex.entries
          .map((entry) => FilterOption(value: entry.key, label: entry.value))
          .toList(),
      isSelected: (value) => context
          .read<SettingsCubit>()
          .state
          .enabledDifficultyIndices
          .contains(value),
      onSelectAll: (_) =>
          context.read<SettingsCubit>().updateEnabledDifficulties(
            AppConstants.defaultEnabledDifficultyIndices.toSet(),
          ),
      onToggle: _toggleDifficulty,
    );
  }

  void _showVersionSheet() {
    if (_isLoadingVersions) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('Loading versions...')));
      return;
    }
    if (_versionOptions.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No version data available')),
      );
      return;
    }

    showFilterBottomSheet<int>(
      context: context,
      title: 'Versions',
      options: _versionOptions
          .map(
            (version) => FilterOption(
              value: version.versionIndex,
              label: version.versionName,
              subtitle: '${version.songCount} songs',
            ),
          )
          .toList(),
      isSelected: (value) {
        final selected = context
            .read<SettingsCubit>()
            .state
            .includeVersionIndices;
        return selected == null || selected.contains(value);
      },
      onSelectAll: (_) =>
          context.read<SettingsCubit>().updateIncludeVersionIndices(null),
      onSelectNone: (_) =>
          context.read<SettingsCubit>().updateIncludeVersionIndices(<int>{}),
      onToggle: _toggleVersion,
    );
  }

  void _triggerRangeAnimation() {
    _rangeAnimController.forward().then((_) => _rangeAnimController.reverse());
  }

  @override
  Widget build(BuildContext context) {
    return MultiBlocListener(
      listeners: [
        BlocListener<HardwareInputCubit, HardwareInputState>(
          listener: (context, state) {
            if (state is IncrementRangeState ||
                state is DecrementRangeState ||
                state is TriggerRandomState) {
              _onHardwareInput(state);
            }
          },
        ),
        BlocListener<LevelRangeCubit, LevelRangeState>(
          listener: (context, state) {
            if (_previousRangeState != null &&
                (_previousRangeState!.start != state.start ||
                    _previousRangeState!.end != state.end)) {
              _triggerRangeAnimation();
            }
            _previousRangeState = state;
          },
        ),
        BlocListener<SettingsCubit, SettingsState>(
          listenWhen: (previous, current) =>
              previous.songInfoServerUrl != current.songInfoServerUrl,
          listener: (context, state) {
            _loadVersionOptions(state.songInfoServerUrl);
          },
        ),
      ],
      child: Scaffold(
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(
              AppSpacing.screenPadding,
              AppSpacing.sm,
              AppSpacing.screenPadding,
              AppSpacing.sm,
            ),
            child: Column(
              children: [
                // ── Row 1: LV + GAP controls + Settings gear ──
                BlocBuilder<LevelRangeCubit, LevelRangeState>(
                  builder: (context, levelState) {
                    return Row(
                      children: [
                        Expanded(
                          child: ScaleTransition(
                            scale: _rangeScaleAnimation,
                            child: LevelGapControls(
                              levelText: levelState.start.toStringAsFixed(1),
                              gapText: levelState.gap.toStringAsFixed(1),
                              onLevelDecrement: () => context
                                  .read<LevelRangeCubit>()
                                  .decrementLevel(),
                              onLevelIncrement: () => context
                                  .read<LevelRangeCubit>()
                                  .incrementLevel(),
                              onGapDecrement: () => context
                                  .read<LevelRangeCubit>()
                                  .decrementGap(),
                              onGapIncrement: () => context
                                  .read<LevelRangeCubit>()
                                  .incrementGap(),
                            ),
                          ),
                        ),
                        const SizedBox(width: AppSpacing.sm),
                        SizedBox(
                          width: 40,
                          height: 40,
                          child: IconButton(
                            style: IconButton.styleFrom(
                              backgroundColor: AppColors.surfaceElevated,
                              padding: EdgeInsets.zero,
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(10),
                                side: BorderSide(
                                  color: AppColors.accentPrimary.withValues(
                                    alpha: 0.3,
                                  ),
                                ),
                              ),
                            ),
                            onPressed: () => Navigator.pushNamed(
                              context,
                              SettingsScreen.routeName,
                            ),
                            icon: const Icon(
                              Icons.settings_rounded,
                              size: 18,
                              color: AppColors.textSecondary,
                            ),
                          ),
                        ),
                      ],
                    );
                  },
                ),
                const SizedBox(height: AppSpacing.xs + 2), // 6px
                // ── Row 2: Filter chips ──
                BlocBuilder<SettingsCubit, SettingsState>(
                  builder: (context, settingsState) {
                    return FilterChipBar(
                      chartTypeLabel: _buildChartTypeLabel(settingsState),
                      difficultyLabel: _buildDifficultyLabel(settingsState),
                      versionLabel: _buildVersionLabel(settingsState),
                      isVersionLoading: _isLoadingVersions,
                      onChartTypeTap: () => _showChartTypeSheet(settingsState),
                      onDifficultyTap: () =>
                          _showDifficultySheet(settingsState),
                      onVersionTap: _showVersionSheet,
                    );
                  },
                ),
                const SizedBox(height: AppSpacing.sm),

                // ── Song display (fills remaining space) ──
                Expanded(
                  child: BlocBuilder<SettingsCubit, SettingsState>(
                    buildWhen: (previous, current) =>
                        previous.showLevel != current.showLevel ||
                        previous.showUserLevel != current.showUserLevel,
                    builder: (context, settingsState) {
                      return BlocBuilder<SongCubit, SongState>(
                        builder: (context, state) {
                          return SongDisplaySection(
                            state: state,
                            showLevel: settingsState.showLevel,
                            showUserLevel: settingsState.showUserLevel,
                          );
                        },
                      );
                    },
                  ),
                ),
                const SizedBox(height: AppSpacing.sm),

                // ── Bottom: Full-width RANDOM button ──
                BlocBuilder<SongCubit, SongState>(
                  builder: (context, songState) {
                    return RandomButton(
                      isLoading: songState is SongLoading,
                      onPressed: _fetchRandomSong,
                    );
                  },
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
