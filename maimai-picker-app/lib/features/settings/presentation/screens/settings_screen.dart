import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:dio/dio.dart';

import '../../bloc/settings/settings_cubit.dart';
import '../../bloc/settings/settings_state.dart';

/// Settings screen for app configuration.
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  static const String routeName = '/settings';

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  static const _defaultSongInfoServerUrl = 'http://localhost:3001';
  static const _defaultRecordCollectorServerUrl = 'http://localhost:3000';

  late TextEditingController _songInfoUrlController;
  late TextEditingController _recordCollectorUrlController;
  bool _hasChanges = false;

  // Song Info Server health check state
  bool _isCheckingSongInfoHealth = false;
  bool? _songInfoHealthOk;
  String? _songInfoHealthMessage;

  // Record Collector Server health check state
  bool _isCheckingRecordCollectorHealth = false;
  bool? _recordCollectorHealthOk;
  String? _recordCollectorHealthMessage;

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
      // Reset health status when URL changes
      _songInfoHealthMessage = null;
      _songInfoHealthOk = null;
      _recordCollectorHealthMessage = null;
      _recordCollectorHealthOk = null;
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
    await cubit.updateRecordCollectorServerUrl(recordCollectorUrl);

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

  Future<void> _checkSongInfoHealth() async {
    final baseUrl = _normalizeBaseUrl(_songInfoUrlController.text);

    if (baseUrl.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Song Info Server URL cannot be empty'),
          backgroundColor: Colors.red,
        ),
      );
      return;
    }

    setState(() {
      _isCheckingSongInfoHealth = true;
      _songInfoHealthOk = null;
      _songInfoHealthMessage = null;
    });

    await _performHealthCheck(
      baseUrl: baseUrl,
      onSuccess: (message) {
        if (mounted) {
          setState(() {
            _songInfoHealthOk = true;
            _songInfoHealthMessage = message;
          });
        }
      },
      onError: (message) {
        if (mounted) {
          setState(() {
            _songInfoHealthOk = false;
            _songInfoHealthMessage = message;
          });
        }
      },
      onComplete: () {
        if (mounted) {
          setState(() {
            _isCheckingSongInfoHealth = false;
          });
        }
      },
    );
  }

  Future<void> _checkRecordCollectorHealth() async {
    final baseUrl = _normalizeBaseUrl(_recordCollectorUrlController.text);

    if (baseUrl.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Record Collector Server URL is empty'),
          backgroundColor: Colors.orange,
        ),
      );
      return;
    }

    setState(() {
      _isCheckingRecordCollectorHealth = true;
      _recordCollectorHealthOk = null;
      _recordCollectorHealthMessage = null;
    });

    await _performHealthCheck(
      baseUrl: baseUrl,
      onSuccess: (message) {
        if (mounted) {
          setState(() {
            _recordCollectorHealthOk = true;
            _recordCollectorHealthMessage = message;
          });
        }
      },
      onError: (message) {
        if (mounted) {
          setState(() {
            _recordCollectorHealthOk = false;
            _recordCollectorHealthMessage = message;
          });
        }
      },
      onComplete: () {
        if (mounted) {
          setState(() {
            _isCheckingRecordCollectorHealth = false;
          });
        }
      },
    );
  }

  Future<void> _performHealthCheck({
    required String baseUrl,
    required void Function(String message) onSuccess,
    required void Function(String message) onError,
    required void Function() onComplete,
  }) async {
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
        onSuccess('Healthy (HTTP 200)');
      } else {
        onError('Unexpected response (HTTP $statusCode)');
      }
    } on DioException catch (e) {
      onError('Connection failed: ${e.message ?? 'unknown error'}');
    } catch (e) {
      onError('Unexpected error: $e');
    } finally {
      onComplete();
    }
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
                hintText: _defaultSongInfoServerUrl,
                theme: theme,
                colorScheme: colorScheme,
              ),
              const SizedBox(height: 8),

              // Song Info Health Check Button
              _buildHealthCheckButton(
                isChecking: _isCheckingSongInfoHealth,
                onPressed: _checkSongInfoHealth,
                healthOk: _songInfoHealthOk,
                healthMessage: _songInfoHealthMessage,
                theme: theme,
                colorScheme: colorScheme,
              ),
              const SizedBox(height: 20),

              // Record Collector Server URL TextField (optional)
              _buildUrlTextField(
                controller: _recordCollectorUrlController,
                labelText: 'Record Collector Server URL (optional)',
                hintText: _defaultRecordCollectorServerUrl,
                theme: theme,
                colorScheme: colorScheme,
              ),
              const SizedBox(height: 8),

              // Record Collector Health Check Button
              _buildHealthCheckButton(
                isChecking: _isCheckingRecordCollectorHealth,
                onPressed: _checkRecordCollectorHealth,
                healthOk: _recordCollectorHealthOk,
                healthMessage: _recordCollectorHealthMessage,
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
              const SizedBox(height: 32),

              // ─────────────────────────────────────────────────────────────
              // Display Settings
              // ─────────────────────────────────────────────────────────────
              Text(
                'DISPLAY',
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
                        'Shown next to internal level as \u26a1 13.7 (A).\n'
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
                      _songInfoUrlController.text = _defaultSongInfoServerUrl;
                      _recordCollectorUrlController.text =
                          _defaultRecordCollectorServerUrl;
                      setState(() {
                        _hasChanges = false;
                        _songInfoHealthOk = null;
                        _songInfoHealthMessage = null;
                        _recordCollectorHealthOk = null;
                        _recordCollectorHealthMessage = null;
                      });
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

  Widget _buildHealthCheckButton({
    required bool isChecking,
    required VoidCallback onPressed,
    required bool? healthOk,
    required String? healthMessage,
    required ThemeData theme,
    required ColorScheme colorScheme,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          height: 40,
          child: OutlinedButton(
            onPressed: isChecking ? null : onPressed,
            style: OutlinedButton.styleFrom(
              foregroundColor: colorScheme.secondary,
              side: BorderSide(color: colorScheme.secondary, width: 1.5),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              padding: const EdgeInsets.symmetric(horizontal: 16),
            ),
            child: isChecking
                ? Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: colorScheme.secondary,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        'Checking...',
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: colorScheme.secondary,
                        ),
                      ),
                    ],
                  )
                : Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.network_check,
                        size: 18,
                        color: colorScheme.secondary,
                      ),
                      const SizedBox(width: 6),
                      Text(
                        'Health Check',
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: colorScheme.secondary,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
          ),
        ),
        if (healthMessage != null) ...[
          const SizedBox(height: 6),
          Row(
            children: [
              Icon(
                healthOk == true ? Icons.check_circle : Icons.error,
                size: 16,
                color: healthOk == true ? colorScheme.primary : colorScheme.error,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  healthMessage,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: healthOk == true
                        ? colorScheme.primary
                        : colorScheme.error,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
        ],
      ],
    );
  }
}
