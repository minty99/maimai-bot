import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../../core/constants/app_constants.dart';
import 'settings_state.dart';

/// Cubit for managing application settings.
///
/// Handles server URL configuration, random filters, and display toggles
/// with SharedPreferences persistence.
class SettingsCubit extends Cubit<SettingsState> {
  SettingsCubit()
    : super(
        SettingsState(
          songInfoServerUrl: AppConstants.defaultSongInfoServerUrl,
          recordCollectorServerUrl:
              AppConstants.defaultRecordCollectorServerUrl,
          enabledChartTypes: AppConstants.defaultEnabledChartTypes.toSet(),
          enabledDifficultyIndices: AppConstants.defaultEnabledDifficultyIndices
              .toSet(),
          includeVersionIndices: null,
          showLevel: AppConstants.defaultShowLevel,
          showUserLevel: AppConstants.defaultShowUserLevel,
        ),
      );

  static const String _songInfoServerUrlKey = 'song_info_server_url';
  static const String _recordCollectorServerUrlKey =
      'record_collector_server_url';
  static const String _enabledChartTypesKey = 'enabled_chart_types';
  static const String _enabledDifficultiesKey = 'enabled_difficulties';
  static const String _includeVersionIndicesKey = 'include_version_indices';
  static const String _showLevelKey = 'show_level';
  static const String _showUserLevelKey = 'show_user_level';

  /// Initialize settings from SharedPreferences.
  ///
  /// Call this during app startup to load persisted settings.
  Future<void> initialize() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final savedSongInfoUrl = prefs.getString(_songInfoServerUrlKey);
      final savedRecordCollectorUrl = prefs.getString(
        _recordCollectorServerUrlKey,
      );

      final enabledChartTypes = _readStringSet(
        prefs,
        _enabledChartTypesKey,
        AppConstants.defaultEnabledChartTypes,
      );
      final enabledDifficultyIndices = _readIntSetWithFallback(
        prefs,
        _enabledDifficultiesKey,
        AppConstants.defaultEnabledDifficultyIndices,
      );
      final includeVersionIndices = _readOptionalIntSet(
        prefs,
        _includeVersionIndicesKey,
      );
      final showLevel =
          prefs.getBool(_showLevelKey) ?? AppConstants.defaultShowLevel;
      final savedShowUserLevel =
          prefs.getBool(_showUserLevelKey) ?? AppConstants.defaultShowUserLevel;
      final showUserLevel = showLevel ? savedShowUserLevel : false;

      // Normalize persisted state so user level is always off when level is off.
      if (!showLevel && savedShowUserLevel) {
        await prefs.setBool(_showUserLevelKey, false);
      }

      emit(
        SettingsState(
          songInfoServerUrl:
              (savedSongInfoUrl != null && savedSongInfoUrl.isNotEmpty)
              ? savedSongInfoUrl
              : AppConstants.defaultSongInfoServerUrl,
          recordCollectorServerUrl:
              (savedRecordCollectorUrl != null &&
                  savedRecordCollectorUrl.isNotEmpty)
              ? savedRecordCollectorUrl
              : AppConstants.defaultRecordCollectorServerUrl,
          enabledChartTypes: enabledChartTypes,
          enabledDifficultyIndices: enabledDifficultyIndices,
          includeVersionIndices: includeVersionIndices,
          showLevel: showLevel,
          showUserLevel: showUserLevel,
        ),
      );
    } catch (e) {
      // If loading fails, keep default state
      // Could emit an error state here if needed
    }
  }

  /// Update the Song Info Server URL and persist it.
  ///
  /// [url] - The new Song Info Server URL.
  Future<void> updateSongInfoServerUrl(String url) async {
    if (url.isEmpty) {
      return;
    }

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_songInfoServerUrlKey, url);
      emit(state.copyWith(songInfoServerUrl: url));
    } catch (e) {
      // Handle persistence error
      // Could emit an error state here if needed
    }
  }

  /// Update the Record Collector Server URL and persist it.
  ///
  /// [url] - The new Record Collector Server URL. Can be empty to disable
  /// personal score integration (record collector is optional).
  Future<void> updateRecordCollectorServerUrl(String url) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      if (url.isEmpty) {
        await prefs.remove(_recordCollectorServerUrlKey);
      } else {
        await prefs.setString(_recordCollectorServerUrlKey, url);
      }
      emit(state.copyWith(recordCollectorServerUrl: url));
    } catch (e) {
      // Handle persistence error
      // Could emit an error state here if needed
    }
  }

  /// Update chart type filters.
  Future<void> updateEnabledChartTypes(Set<String> chartTypes) async {
    if (chartTypes.isEmpty) {
      return;
    }

    final normalized = chartTypes
        .map((value) => value.trim().toUpperCase())
        .where((value) => value.isNotEmpty)
        .toSet();
    if (normalized.isEmpty) {
      return;
    }

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setStringList(
        _enabledChartTypesKey,
        _sortedStrings(normalized),
      );
      emit(state.copyWith(enabledChartTypes: normalized));
    } catch (e) {
      // Handle persistence error
    }
  }

  /// Update difficulty filters.
  Future<void> updateEnabledDifficulties(Set<int> difficultyIndices) async {
    if (difficultyIndices.isEmpty) {
      return;
    }

    final normalized = {...difficultyIndices};

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setStringList(
        _enabledDifficultiesKey,
        _sortedInts(normalized).map((value) => value.toString()).toList(),
      );
      emit(state.copyWith(enabledDifficultyIndices: normalized));
    } catch (e) {
      // Handle persistence error
    }
  }

  /// Update included version indices.
  ///
  /// - null: include all versions (do not send include_versions)
  /// - non-null: include only those indices
  Future<void> updateIncludeVersionIndices(Set<int>? indices) async {
    final normalized = indices == null ? null : {...indices};

    try {
      final prefs = await SharedPreferences.getInstance();
      if (normalized == null) {
        await prefs.remove(_includeVersionIndicesKey);
      } else {
        await prefs.setStringList(
          _includeVersionIndicesKey,
          _sortedInts(normalized).map((value) => value.toString()).toList(),
        );
      }

      emit(state.copyWith(includeVersionIndices: normalized));
    } catch (e) {
      // Handle persistence error
    }
  }

  /// Update whether display level should be shown.
  Future<void> updateShowLevel(bool showLevel) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(_showLevelKey, showLevel);
      if (!showLevel) {
        await prefs.setBool(_showUserLevelKey, false);
        emit(state.copyWith(showLevel: false, showUserLevel: false));
      } else {
        emit(state.copyWith(showLevel: true));
      }
    } catch (e) {
      // Handle persistence error
    }
  }

  /// Update whether user level label should be shown.
  Future<void> updateShowUserLevel(bool showUserLevel) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(_showUserLevelKey, showUserLevel);
      emit(state.copyWith(showUserLevel: showUserLevel));
    } catch (e) {
      // Handle persistence error
    }
  }

  /// Reset both server URLs to defaults.
  Future<void> resetServerUrls() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.remove(_songInfoServerUrlKey);
      await prefs.remove(_recordCollectorServerUrlKey);
      emit(
        state.copyWith(
          songInfoServerUrl: AppConstants.defaultSongInfoServerUrl,
          recordCollectorServerUrl:
              AppConstants.defaultRecordCollectorServerUrl,
        ),
      );
    } catch (e) {
      // Handle persistence error
    }
  }

  Set<String> _readStringSet(
    SharedPreferences prefs,
    String key,
    List<String> fallback,
  ) {
    final saved = prefs.getStringList(key);
    if (saved == null || saved.isEmpty) {
      return fallback.toSet();
    }

    final values = saved
        .map((value) => value.trim())
        .where((value) => value.isNotEmpty)
        .toSet();
    if (values.isEmpty) {
      return fallback.toSet();
    }

    return values;
  }

  Set<int>? _readOptionalIntSet(SharedPreferences prefs, String key) {
    final saved = prefs.getStringList(key);
    if (saved == null) {
      return null;
    }

    final values = saved
        .map((value) => int.tryParse(value.trim()))
        .whereType<int>()
        .toSet();

    if (values.isEmpty) {
      return null;
    }

    return values;
  }

  Set<int> _readIntSetWithFallback(
    SharedPreferences prefs,
    String key,
    List<int> fallback,
  ) {
    final saved = prefs.getStringList(key);
    if (saved == null || saved.isEmpty) {
      return fallback.toSet();
    }

    final parsed = saved
        .map((value) => int.tryParse(value.trim()))
        .whereType<int>()
        .toSet();
    if (parsed.isEmpty) {
      return fallback.toSet();
    }
    return parsed;
  }

  List<String> _sortedStrings(Set<String> values) {
    final sorted = values.toList()..sort();
    return sorted;
  }

  List<int> _sortedInts(Set<int> values) {
    final sorted = values.toList()..sort();
    return sorted;
  }
}
