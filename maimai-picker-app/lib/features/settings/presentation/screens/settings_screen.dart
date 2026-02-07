import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:dio/dio.dart';

import '../../../../core/constants/app_constants.dart';
import '../../bloc/settings/settings_cubit.dart';
import '../../bloc/settings/settings_state.dart';

/// Settings screen for app configuration.
///
/// Features:
/// - Song Info Server URL configuration with TextField
/// - Record Collector Server URL configuration with TextField
/// - Save button with SnackBar feedback
/// - Info card with setup instructions
/// - Large touch targets for glove-friendly interaction
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  static const String routeName = '/settings';

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late TextEditingController _songInfoUrlController;
  late TextEditingController _recordCollectorUrlController;
  bool _hasChanges = false;
  bool _isCheckingHealth = false;
  bool? _healthOk;
  String? _healthMessage;
  bool _isLoadingVersions = false;
  String? _versionsError;
  List<_VersionOption> _versionOptions = const [];

  @override
  void initState() {
    super.initState();
    final state = context.read<SettingsCubit>().state;
    _songInfoUrlController = TextEditingController(
      text: state.songInfoServerUrl,
    );
    _recordCollectorUrlController = TextEditingController(
      text: state.recordCollectorServerUrl,
    );
    _songInfoUrlController.addListener(_onTextChanged);
    _recordCollectorUrlController.addListener(_onTextChanged);
    _loadVersionOptions(baseUrl: state.songInfoServerUrl);
  }

  @override
  void dispose() {
    _songInfoUrlController.removeListener(_onTextChanged);
    _recordCollectorUrlController.removeListener(_onTextChanged);
    _songInfoUrlController.dispose();
    _recordCollectorUrlController.dispose();
    super.dispose();
  }

  void _onTextChanged() {
    final state = context.read<SettingsCubit>().state;
    setState(() {
      _hasChanges =
          _songInfoUrlController.text != state.songInfoServerUrl ||
          _recordCollectorUrlController.text != state.recordCollectorServerUrl;
      _healthMessage = null;
      _healthOk = null;
    });
  }

  Future<void> _saveSettings() async {
    final songInfoUrl = _songInfoUrlController.text.trim();
    final recordCollectorUrl = _recordCollectorUrlController.text.trim();

    if (songInfoUrl.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Song Info Server URL is required'),
          backgroundColor: Colors.red,
        ),
      );
      return;
    }

    final cubit = context.read<SettingsCubit>();
    await cubit.updateSongInfoServerUrl(songInfoUrl);
    // Record Collector Server URL is optional - save even if empty
    await cubit.updateRecordCollectorServerUrl(recordCollectorUrl);
    await _loadVersionOptions(baseUrl: songInfoUrl);

    if (mounted) {
      setState(() {
        _hasChanges = false;
      });

      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: const Text('Settings saved'),
          backgroundColor: Theme.of(context).colorScheme.primary,
        ),
      );
    }
  }

  String _normalizeBaseUrl(String rawUrl) {
    final trimmed = rawUrl.trim();
    if (trimmed.endsWith('/')) {
      return trimmed.substring(0, trimmed.length - 1);
    }
    return trimmed;
  }

  Future<void> _checkHealth() async {
    final rawUrl = _songInfoUrlController.text;
    final baseUrl = _normalizeBaseUrl(rawUrl);

    if (baseUrl.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('URL cannot be empty'),
          backgroundColor: Colors.red,
        ),
      );
      return;
    }

    setState(() {
      _isCheckingHealth = true;
      _healthOk = null;
      _healthMessage = null;
    });

    final dio = Dio(
      BaseOptions(
        connectTimeout: const Duration(seconds: 5),
        receiveTimeout: const Duration(seconds: 5),
      ),
    );

    try {
      final response = await dio.get<Map<String, dynamic>>('$baseUrl/health');

      final statusCode = response.statusCode ?? 0;
      final status = response.data?['status']?.toString();

      if (statusCode == 200 && status == 'ok') {
        if (!mounted) return;
        setState(() {
          _healthOk = true;
          _healthMessage = 'Healthy (HTTP 200)';
        });
      } else {
        if (!mounted) return;
        setState(() {
          _healthOk = false;
          _healthMessage = 'Unexpected response (HTTP $statusCode)';
        });
      }
    } on DioException catch (e) {
      if (!mounted) return;
      setState(() {
        _healthOk = false;
        _healthMessage = 'Connection failed: ${e.message ?? 'unknown error'}';
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _healthOk = false;
        _healthMessage = 'Unexpected error: $e';
      });
    } finally {
      if (mounted) {
        setState(() {
          _isCheckingHealth = false;
        });
      }
    }

    if (!mounted) return;
    final message = _healthMessage ?? 'Health check completed';
    final backgroundColor = _healthOk == true
        ? Theme.of(context).colorScheme.primary
        : Theme.of(context).colorScheme.error;

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: backgroundColor),
    );

    if (_healthOk == true) {
      await _loadVersionOptions(baseUrl: baseUrl);
    }
  }

  Future<void> _loadVersionOptions({String? baseUrl}) async {
    final normalized = _normalizeBaseUrl(
      baseUrl ?? _songInfoUrlController.text,
    );
    if (normalized.isEmpty) {
      if (!mounted) return;
      setState(() {
        _versionOptions = const [];
        _versionsError = 'Song Info Server URL is empty';
      });
      return;
    }

    setState(() {
      _isLoadingVersions = true;
      _versionsError = null;
    });

    final dio = Dio(
      BaseOptions(
        connectTimeout: const Duration(seconds: 5),
        receiveTimeout: const Duration(seconds: 5),
      ),
    );

    try {
      final response = await dio.get<Map<String, dynamic>>(
        '$normalized/api/songs/versions',
      );
      final rawVersions = response.data?['versions'] as List<dynamic>? ?? [];
      final parsed =
          rawVersions
              .map(
                (raw) => _VersionOption.fromJson(raw as Map<String, dynamic>),
              )
              .where((version) => version.versionIndex >= 0)
              .toList()
            ..sort((a, b) => a.versionIndex.compareTo(b.versionIndex));

      if (!mounted) return;
      setState(() {
        _versionOptions = parsed;
        _versionsError = null;
      });
    } on DioException catch (e) {
      if (!mounted) return;
      setState(() {
        _versionsError =
            'Failed to load versions: ${e.message ?? "network error"}';
        _versionOptions = const [];
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _versionsError = 'Failed to load versions: $e';
        _versionOptions = const [];
      });
    } finally {
      if (mounted) {
        setState(() {
          _isLoadingVersions = false;
        });
      }
    }
  }

  Future<void> _toggleChartType(String chartType, bool selected) async {
    final state = context.read<SettingsCubit>().state;
    final next = {...state.enabledChartTypes};
    if (selected) {
      next.add(chartType);
    } else {
      if (next.length == 1 && next.contains(chartType)) {
        _showSimpleSnackBar('At least one chart type must remain enabled');
        return;
      }
      next.remove(chartType);
    }

    await context.read<SettingsCubit>().updateEnabledChartTypes(next);
  }

  Future<void> _toggleDifficulty(int difficultyIndex, bool selected) async {
    final state = context.read<SettingsCubit>().state;
    final next = {...state.enabledDifficultyIndices};
    if (selected) {
      next.add(difficultyIndex);
    } else {
      if (next.length == 1 && next.contains(difficultyIndex)) {
        _showSimpleSnackBar('At least one difficulty must remain enabled');
        return;
      }
      next.remove(difficultyIndex);
    }

    await context.read<SettingsCubit>().updateEnabledDifficulties(next);
  }

  Future<void> _selectAllVersions() async {
    await context.read<SettingsCubit>().updateIncludeVersionIndices(null);
  }

  Future<void> _deselectAllVersions() async {
    await context.read<SettingsCubit>().updateIncludeVersionIndices(<int>{});
  }

  Future<void> _toggleVersion(int versionIndex, bool selected) async {
    final cubit = context.read<SettingsCubit>();
    final state = cubit.state;
    final allVersionIndices = _versionOptions
        .map((version) => version.versionIndex)
        .toSet();

    if (allVersionIndices.isEmpty) {
      return;
    }

    final current = state.includeVersionIndices;
    if (selected) {
      if (current == null) {
        return;
      }
      final next = {...current, versionIndex};
      if (next.length == allVersionIndices.length) {
        await cubit.updateIncludeVersionIndices(null);
      } else {
        await cubit.updateIncludeVersionIndices(next);
      }
      return;
    }

    final base = current == null ? {...allVersionIndices} : {...current};
    base.remove(versionIndex);
    await cubit.updateIncludeVersionIndices(base);
  }

  void _showSimpleSnackBar(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          iconSize: 28,
          onPressed: () => Navigator.of(context).pop(),
        ),
      ),
      body: BlocBuilder<SettingsCubit, SettingsState>(
        builder: (context, state) {
          return ListView(
            padding: const EdgeInsets.all(24.0),
            children: [
              // ─────────────────────────────────────────────────────────────
              // Server URLs Section
              // ─────────────────────────────────────────────────────────────
              Text(
                'CONNECTION',
                style: theme.textTheme.labelLarge?.copyWith(
                  color: colorScheme.primary,
                  letterSpacing: 2,
                ),
              ),
              const SizedBox(height: 16),

              // Song Info Server URL TextField
              _buildUrlTextField(
                controller: _songInfoUrlController,
                labelText: 'Song Info Server URL',
                hintText: AppConstants.defaultSongInfoServerUrl,
                theme: theme,
                colorScheme: colorScheme,
              ),
              const SizedBox(height: 16),

              // Record Collector Server URL TextField (optional)
              _buildUrlTextField(
                controller: _recordCollectorUrlController,
                labelText: 'Record Collector Server URL (optional)',
                hintText: AppConstants.defaultRecordCollectorServerUrl,
                theme: theme,
                colorScheme: colorScheme,
              ),
              const SizedBox(height: 24),

              // Save Button
              SizedBox(
                width: double.infinity,
                height: 60,
                child: FilledButton(
                  onPressed: _hasChanges ? _saveSettings : null,
                  style: FilledButton.styleFrom(
                    backgroundColor: colorScheme.primary,
                    foregroundColor: colorScheme.onPrimary,
                    disabledBackgroundColor: colorScheme.primary.withValues(
                      alpha: 0.3,
                    ),
                    disabledForegroundColor: colorScheme.onPrimary.withValues(
                      alpha: 0.5,
                    ),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(16),
                    ),
                  ),
                  child: Text(
                    'SAVE',
                    style: theme.textTheme.titleLarge?.copyWith(
                      color: _hasChanges
                          ? colorScheme.onPrimary
                          : colorScheme.onPrimary.withValues(alpha: 0.5),
                      fontWeight: FontWeight.bold,
                      letterSpacing: 2,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 16),

              // Health Check Button
              SizedBox(
                width: double.infinity,
                height: 52,
                child: FilledButton(
                  onPressed: _isCheckingHealth ? null : _checkHealth,
                  style: FilledButton.styleFrom(
                    backgroundColor: colorScheme.secondary,
                    foregroundColor: colorScheme.onSecondary,
                    disabledBackgroundColor: colorScheme.secondary.withValues(
                      alpha: 0.3,
                    ),
                    disabledForegroundColor: colorScheme.onSecondary.withValues(
                      alpha: 0.5,
                    ),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(16),
                    ),
                  ),
                  child: _isCheckingHealth
                      ? Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            SizedBox(
                              width: 20,
                              height: 20,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: colorScheme.onSecondary,
                              ),
                            ),
                            const SizedBox(width: 12),
                            Text(
                              'CHECKING...',
                              style: theme.textTheme.labelLarge?.copyWith(
                                color: colorScheme.onSecondary,
                                letterSpacing: 1,
                              ),
                            ),
                          ],
                        )
                      : Text(
                          'HEALTH CHECK',
                          style: theme.textTheme.labelLarge?.copyWith(
                            color: colorScheme.onSecondary,
                            letterSpacing: 1,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                ),
              ),
              if (_healthMessage != null) ...[
                const SizedBox(height: 12),
                Text(
                  _healthMessage!,
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: _healthOk == true
                        ? colorScheme.primary
                        : colorScheme.error,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
              const SizedBox(height: 32),

              // ─────────────────────────────────────────────────────────────
              // Random Filters Section
              // ─────────────────────────────────────────────────────────────
              Text(
                'RANDOM FILTERS',
                style: theme.textTheme.labelLarge?.copyWith(
                  color: colorScheme.primary,
                  letterSpacing: 2,
                ),
              ),
              const SizedBox(height: 16),

              Card(
                elevation: 4,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(20),
                  side: BorderSide(
                    color: colorScheme.outline.withValues(alpha: 0.3),
                    width: 1,
                  ),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Chart Type',
                        style: theme.textTheme.titleMedium?.copyWith(
                          color: colorScheme.onSurface,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 10),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: AppConstants.defaultEnabledChartTypes.map((
                          value,
                        ) {
                          return FilterChip(
                            label: Text(value),
                            selected: state.enabledChartTypes.contains(value),
                            onSelected: (selected) {
                              _toggleChartType(value, selected);
                            },
                          );
                        }).toList(),
                      ),
                      const SizedBox(height: 16),
                      Text(
                        'Difficulty',
                        style: theme.textTheme.titleMedium?.copyWith(
                          color: colorScheme.onSurface,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 10),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: AppConstants.difficultyLabelsByIndex.entries
                            .map((entry) {
                              final difficultyIndex = entry.key;
                              final difficultyLabel = entry.value;
                              return FilterChip(
                                label: Text(difficultyLabel),
                                selected: state.enabledDifficultyIndices
                                    .contains(difficultyIndex),
                                onSelected: (selected) {
                                  _toggleDifficulty(difficultyIndex, selected);
                                },
                              );
                            })
                            .toList(),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),

              Card(
                elevation: 4,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(20),
                  side: BorderSide(
                    color: colorScheme.outline.withValues(alpha: 0.3),
                    width: 1,
                  ),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Text(
                            'Versions',
                            style: theme.textTheme.titleMedium?.copyWith(
                              color: colorScheme.onSurface,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          const Spacer(),
                          TextButton(
                            onPressed: _selectAllVersions,
                            child: const Text('SELECT ALL'),
                          ),
                          TextButton(
                            onPressed: _deselectAllVersions,
                            child: const Text('SELECT NONE'),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      if (_isLoadingVersions)
                        const Center(child: CircularProgressIndicator()),
                      if (_versionsError != null) ...[
                        Text(
                          _versionsError!,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: colorScheme.error,
                          ),
                        ),
                        const SizedBox(height: 8),
                      ],
                      if (!_isLoadingVersions && _versionOptions.isEmpty)
                        Text(
                          'No version data loaded',
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      if (_versionOptions.isNotEmpty) ...[
                        ..._versionOptions.map((version) {
                          final selected = state.includeVersionIndices == null
                              ? true
                              : state.includeVersionIndices!.contains(
                                  version.versionIndex,
                                );
                          return CheckboxListTile(
                            dense: true,
                            contentPadding: EdgeInsets.zero,
                            value: selected,
                            onChanged: (checked) {
                              if (checked == null) return;
                              _toggleVersion(version.versionIndex, checked);
                            },
                            title: Text(
                              '#${version.versionIndex} ${version.versionName}',
                            ),
                            subtitle: Text('${version.songCount} songs'),
                          );
                        }),
                      ],
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),

              Card(
                elevation: 4,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(20),
                  side: BorderSide(
                    color: colorScheme.outline.withValues(alpha: 0.3),
                    width: 1,
                  ),
                ),
                child: Column(
                  children: [
                    SwitchListTile(
                      title: const Text('Show Level'),
                      subtitle: const Text('Display level text like 13+'),
                      value: state.showLevel,
                      onChanged: (value) {
                        context.read<SettingsCubit>().updateShowLevel(value);
                      },
                    ),
                    SwitchListTile(
                      title: const Text('Show User Level'),
                      subtitle: const Text('Display user level label like (A)'),
                      value: state.showUserLevel,
                      onChanged: state.showLevel
                          ? (value) {
                              context.read<SettingsCubit>().updateShowUserLevel(
                                value,
                              );
                            }
                          : null,
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 32),

              // ─────────────────────────────────────────────────────────────
              // Info Card
              // ─────────────────────────────────────────────────────────────
              Card(
                elevation: 4,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(20),
                  side: BorderSide(
                    color: colorScheme.outline.withValues(alpha: 0.3),
                    width: 1,
                  ),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.info_outline_rounded,
                            color: colorScheme.primary,
                            size: 28,
                          ),
                          const SizedBox(width: 12),
                          Text(
                            'Setup Instructions',
                            style: theme.textTheme.titleLarge?.copyWith(
                              color: colorScheme.onSurface,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      _InfoItem(
                        icon: Icons.computer_rounded,
                        text:
                            'Start Song Info Server (required) and optionally Record Collector Server',
                        theme: theme,
                        colorScheme: colorScheme,
                      ),
                      const SizedBox(height: 12),
                      _InfoItem(
                        icon: Icons.wifi_rounded,
                        text: 'Ensure phone is on the same network',
                        theme: theme,
                        colorScheme: colorScheme,
                      ),
                      const SizedBox(height: 12),
                      _InfoItem(
                        icon: Icons.link_rounded,
                        text: 'Enter your computer\'s local IP address',
                        theme: theme,
                        colorScheme: colorScheme,
                      ),
                      const SizedBox(height: 20),
                      Container(
                        width: double.infinity,
                        padding: const EdgeInsets.all(16),
                        decoration: BoxDecoration(
                          color: colorScheme.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              'Song Info Server (default):',
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              AppConstants.defaultSongInfoServerUrl,
                              style: theme.textTheme.bodyLarge?.copyWith(
                                color: colorScheme.primary,
                                fontFamily: 'monospace',
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const SizedBox(height: 12),
                            Text(
                              'Record Collector Server (optional, default):',
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              AppConstants.defaultRecordCollectorServerUrl,
                              style: theme.textTheme.bodyLarge?.copyWith(
                                color: colorScheme.primary,
                                fontFamily: 'monospace',
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const SizedBox(height: 12),
                            Text(
                              'Example (local network):',
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              'http://192.168.1.100:3001',
                              style: theme.textTheme.bodyLarge?.copyWith(
                                color: colorScheme.secondary,
                                fontFamily: 'monospace',
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 24),

              // ─────────────────────────────────────────────────────────────
              // User Level Guide
              // ─────────────────────────────────────────────────────────────
              Card(
                elevation: 4,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(20),
                  side: BorderSide(
                    color: colorScheme.outline.withValues(alpha: 0.3),
                    width: 1,
                  ),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.leaderboard_rounded,
                            color: colorScheme.primary,
                            size: 28,
                          ),
                          const SizedBox(width: 12),
                          Text(
                            'User Level',
                            style: theme.textTheme.titleLarge?.copyWith(
                              color: colorScheme.onSurface,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      Text(
                        'Shown next to internal level as ⚡ 13.7 (A).\n'
                        'Ranks from highest to lowest:',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Container(
                        width: double.infinity,
                        padding: const EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 12,
                        ),
                        decoration: BoxDecoration(
                          color: colorScheme.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Text(
                          'S  >  A  >  B  >  C  >  D  >  E  >  F',
                          textAlign: TextAlign.center,
                          style: theme.textTheme.titleMedium?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.bold,
                            fontFamily: 'monospace',
                            letterSpacing: 2,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 32),

              // ─────────────────────────────────────────────────────────────
              // Reset Button
              // ─────────────────────────────────────────────────────────────
              SizedBox(
                width: double.infinity,
                height: 52,
                child: OutlinedButton(
                  onPressed: () async {
                    final messenger = ScaffoldMessenger.of(context);
                    await context.read<SettingsCubit>().resetServerUrls();
                    if (mounted) {
                      _songInfoUrlController.text =
                          AppConstants.defaultSongInfoServerUrl;
                      _recordCollectorUrlController.text =
                          AppConstants.defaultRecordCollectorServerUrl;
                      await _loadVersionOptions(
                        baseUrl: AppConstants.defaultSongInfoServerUrl,
                      );
                      messenger.showSnackBar(
                        const SnackBar(
                          content: Text('Reset to default server URLs'),
                        ),
                      );
                    }
                  },
                  style: OutlinedButton.styleFrom(
                    foregroundColor: colorScheme.error,
                    side: BorderSide(color: colorScheme.error, width: 2),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(16),
                    ),
                  ),
                  child: Text(
                    'RESET TO DEFAULT',
                    style: theme.textTheme.labelLarge?.copyWith(
                      color: colorScheme.error,
                      letterSpacing: 1,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 40),

              // ─────────────────────────────────────────────────────────────
              // Version Info
              // ─────────────────────────────────────────────────────────────
              Center(
                child: Text(
                  'Version 1.0.0',
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildUrlTextField({
    required TextEditingController controller,
    required String labelText,
    required String hintText,
    required ThemeData theme,
    required ColorScheme colorScheme,
  }) {
    return TextField(
      controller: controller,
      style: theme.textTheme.bodyLarge?.copyWith(color: colorScheme.onSurface),
      decoration: InputDecoration(
        labelText: labelText,
        labelStyle: TextStyle(
          color: colorScheme.onSurfaceVariant,
          fontSize: 18,
        ),
        hintText: hintText,
        hintStyle: TextStyle(
          color: colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
        ),
        filled: true,
        fillColor: colorScheme.surfaceContainerHighest,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(16),
          borderSide: BorderSide(color: colorScheme.outline, width: 2),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(16),
          borderSide: BorderSide(color: colorScheme.outline, width: 2),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(16),
          borderSide: BorderSide(color: colorScheme.primary, width: 2),
        ),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 20,
          vertical: 20,
        ),
        suffixIcon: controller.text.isNotEmpty
            ? IconButton(
                icon: const Icon(Icons.clear),
                iconSize: 24,
                onPressed: () {
                  controller.clear();
                },
              )
            : null,
      ),
      keyboardType: TextInputType.url,
      autocorrect: false,
    );
  }
}

class _InfoItem extends StatelessWidget {
  const _InfoItem({
    required this.icon,
    required this.text,
    required this.theme,
    required this.colorScheme,
  });

  final IconData icon;
  final String text;
  final ThemeData theme;
  final ColorScheme colorScheme;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 22, color: colorScheme.onSurfaceVariant),
        const SizedBox(width: 12),
        Expanded(
          child: Text(
            text,
            style: theme.textTheme.bodyLarge?.copyWith(
              color: colorScheme.onSurface,
            ),
          ),
        ),
      ],
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
