import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../../core/theme/app_colors.dart';
import '../../../../core/theme/app_motion.dart';
import '../../../../core/theme/app_spacing.dart';
import '../../../../core/theme/app_typography.dart';
import '../../bloc/settings/settings_cubit.dart';
import '../../bloc/settings/settings_state.dart';
import 'health_check_row.dart';

class ServerConnectionSection extends StatefulWidget {
  const ServerConnectionSection({super.key});

  @override
  State<ServerConnectionSection> createState() =>
      _ServerConnectionSectionState();
}

class _ServerConnectionSectionState extends State<ServerConnectionSection> {
  late final TextEditingController _songInfoUrlController;
  late final TextEditingController _recordCollectorUrlController;

  bool _hasChanges = false;
  bool _isCheckingSongInfoHealth = false;
  bool? _songInfoHealthOk;
  String? _songInfoHealthMessage;
  bool _isCheckingRecordCollectorHealth = false;
  bool? _recordCollectorHealthOk;
  String? _recordCollectorHealthMessage;

  @override
  void initState() {
    super.initState();
    final state = context.read<SettingsCubit>().state;
    _songInfoUrlController = TextEditingController(
      text: state.songInfoServerUrl,
    )..addListener(_onTextChanged);
    _recordCollectorUrlController = TextEditingController(
      text: state.recordCollectorServerUrl,
    )..addListener(_onTextChanged);
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
      _songInfoHealthOk = null;
      _songInfoHealthMessage = null;
      _recordCollectorHealthOk = null;
      _recordCollectorHealthMessage = null;
    });
  }

  String _normalizeBaseUrl(String rawUrl) {
    final trimmed = rawUrl.trim();
    if (trimmed.endsWith('/')) {
      return trimmed.substring(0, trimmed.length - 1);
    }
    return trimmed;
  }

  Future<void> _saveSettings() async {
    final songInfoUrl = _songInfoUrlController.text.trim();
    final recordCollectorUrl = _recordCollectorUrlController.text.trim();

    if (songInfoUrl.isEmpty) {
      _showSnackBar(
        message: 'Song Info Server URL is required',
        color: AppColors.error,
      );
      return;
    }

    final cubit = context.read<SettingsCubit>();
    await cubit.updateSongInfoServerUrl(songInfoUrl);
    await cubit.updateRecordCollectorServerUrl(recordCollectorUrl);

    if (!mounted) {
      return;
    }
    setState(() => _hasChanges = false);
    _showSnackBar(message: 'Settings saved', color: AppColors.success);
  }

  Future<void> _checkSongInfoHealth() async {
    final baseUrl = _normalizeBaseUrl(_songInfoUrlController.text);
    if (baseUrl.isEmpty) {
      _showSnackBar(
        message: 'Song Info Server URL cannot be empty',
        color: AppColors.error,
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
        if (!mounted) {
          return;
        }
        setState(() {
          _songInfoHealthOk = true;
          _songInfoHealthMessage = message;
        });
      },
      onError: (message) {
        if (!mounted) {
          return;
        }
        setState(() {
          _songInfoHealthOk = false;
          _songInfoHealthMessage = message;
        });
      },
      onComplete: () {
        if (!mounted) {
          return;
        }
        setState(() => _isCheckingSongInfoHealth = false);
      },
    );
  }

  Future<void> _checkRecordCollectorHealth() async {
    final baseUrl = _normalizeBaseUrl(_recordCollectorUrlController.text);
    if (baseUrl.isEmpty) {
      _showSnackBar(
        message: 'Record Collector Server URL is empty',
        color: AppColors.accentSecondary,
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
        if (!mounted) {
          return;
        }
        setState(() {
          _recordCollectorHealthOk = true;
          _recordCollectorHealthMessage = message;
        });
      },
      onError: (message) {
        if (!mounted) {
          return;
        }
        setState(() {
          _recordCollectorHealthOk = false;
          _recordCollectorHealthMessage = message;
        });
      },
      onComplete: () {
        if (!mounted) {
          return;
        }
        setState(() => _isCheckingRecordCollectorHealth = false);
      },
    );
  }

  Future<void> _performHealthCheck({
    required String baseUrl,
    required void Function(String message) onSuccess,
    required void Function(String message) onError,
    required VoidCallback onComplete,
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

  void _showSnackBar({required String message, required Color color}) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message), backgroundColor: color));
  }

  @override
  Widget build(BuildContext context) {
    return BlocListener<SettingsCubit, SettingsState>(
      listenWhen: (previous, current) =>
          previous.songInfoServerUrl != current.songInfoServerUrl ||
          previous.recordCollectorServerUrl != current.recordCollectorServerUrl,
      listener: (context, state) {
        if (_songInfoUrlController.text != state.songInfoServerUrl) {
          _songInfoUrlController.text = state.songInfoServerUrl;
        }
        if (_recordCollectorUrlController.text !=
            state.recordCollectorServerUrl) {
          _recordCollectorUrlController.text = state.recordCollectorServerUrl;
        }
      },
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'CONNECTION',
            style: AppTypography.textTheme.labelLarge?.copyWith(
              color: AppColors.accentPrimary,
              letterSpacing: 1.8,
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          _UrlFieldCard(
            controller: _songInfoUrlController,
            labelText: 'Song Info Server URL',
            hintText: 'http://localhost:3001',
          ),
          const SizedBox(height: AppSpacing.sm),
          HealthCheckRow(
            isChecking: _isCheckingSongInfoHealth,
            onPressed: _checkSongInfoHealth,
            healthOk: _songInfoHealthOk,
            healthMessage: _songInfoHealthMessage,
          ),
          const SizedBox(height: AppSpacing.lg),
          _UrlFieldCard(
            controller: _recordCollectorUrlController,
            labelText: 'Record Collector Server URL (optional)',
            hintText: 'http://localhost:3000',
          ),
          const SizedBox(height: AppSpacing.sm),
          HealthCheckRow(
            isChecking: _isCheckingRecordCollectorHealth,
            onPressed: _checkRecordCollectorHealth,
            healthOk: _recordCollectorHealthOk,
            healthMessage: _recordCollectorHealthMessage,
          ),
          const SizedBox(height: AppSpacing.xl),
          SizedBox(
            width: double.infinity,
            height: 54,
            child: AnimatedContainer(
              duration: AppMotion.fast,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(16),
                boxShadow: [
                  if (_hasChanges)
                    BoxShadow(
                      color: AppColors.accentPrimary.withValues(alpha: 0.32),
                      blurRadius: 20,
                      spreadRadius: -10,
                    ),
                ],
              ),
              child: FilledButton(
                onPressed: _hasChanges ? _saveSettings : null,
                style: FilledButton.styleFrom(
                  backgroundColor: AppColors.accentPrimary,
                  disabledBackgroundColor: AppColors.accentPrimary.withValues(
                    alpha: 0.35,
                  ),
                  foregroundColor: AppColors.background,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(16),
                  ),
                ),
                child: Text(
                  'SAVE',
                  style: AppTypography.textTheme.labelLarge?.copyWith(
                    color: AppColors.background,
                    letterSpacing: 1.6,
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _UrlFieldCard extends StatelessWidget {
  const _UrlFieldCard({
    required this.controller,
    required this.labelText,
    required this.hintText,
  });

  final TextEditingController controller;
  final String labelText;
  final String hintText;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(AppSpacing.md),
      decoration: BoxDecoration(
        color: AppColors.surfaceElevated,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(
          color: AppColors.accentPrimary.withValues(alpha: 0.55),
        ),
        boxShadow: [
          BoxShadow(
            color: AppColors.accentPrimary.withValues(alpha: 0.16),
            blurRadius: 18,
            spreadRadius: -8,
          ),
        ],
      ),
      child: TextField(
        controller: controller,
        keyboardType: TextInputType.url,
        autocorrect: false,
        style: AppTypography.textTheme.bodyLarge,
        cursorColor: AppColors.accentPrimary,
        decoration: InputDecoration(
          labelText: labelText,
          labelStyle: AppTypography.textTheme.bodySmall?.copyWith(
            color: AppColors.textSecondary,
          ),
          hintText: hintText,
          hintStyle: AppTypography.textTheme.bodySmall?.copyWith(
            color: AppColors.textMuted,
          ),
          filled: true,
          fillColor: AppColors.surface,
          contentPadding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.md,
            vertical: AppSpacing.md,
          ),
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(12),
            borderSide: BorderSide(
              color: AppColors.accentPrimary.withValues(alpha: 0.45),
            ),
          ),
          enabledBorder: OutlineInputBorder(
            borderRadius: BorderRadius.circular(12),
            borderSide: BorderSide(
              color: AppColors.accentPrimary.withValues(alpha: 0.45),
            ),
          ),
          focusedBorder: const OutlineInputBorder(
            borderRadius: BorderRadius.all(Radius.circular(12)),
            borderSide: BorderSide(color: AppColors.accentPrimary, width: 1.5),
          ),
          suffixIcon: controller.text.isEmpty
              ? null
              : IconButton(
                  icon: const Icon(Icons.clear_rounded),
                  color: AppColors.textSecondary,
                  onPressed: controller.clear,
                ),
        ),
      ),
    );
  }
}
