import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../../core/constants/app_constants.dart';
import 'settings_state.dart';

/// Cubit for managing application settings.
///
/// Handles server URL configuration with SharedPreferences persistence.
class SettingsCubit extends Cubit<SettingsState> {
  SettingsCubit()
    : super(
        const SettingsState(
          songInfoServerUrl: AppConstants.defaultSongInfoServerUrl,
          recordCollectorServerUrl:
              AppConstants.defaultRecordCollectorServerUrl,
        ),
      );

  static const String _songInfoServerUrlKey = 'song_info_server_url';
  static const String _recordCollectorServerUrlKey =
      'record_collector_server_url';

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
  /// [url] - The new Record Collector Server URL.
  Future<void> updateRecordCollectorServerUrl(String url) async {
    if (url.isEmpty) {
      return;
    }

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_recordCollectorServerUrlKey, url);
      emit(state.copyWith(recordCollectorServerUrl: url));
    } catch (e) {
      // Handle persistence error
      // Could emit an error state here if needed
    }
  }

  /// Reset both server URLs to defaults.
  Future<void> resetServerUrls() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.remove(_songInfoServerUrlKey);
      await prefs.remove(_recordCollectorServerUrlKey);
      emit(
        const SettingsState(
          songInfoServerUrl: AppConstants.defaultSongInfoServerUrl,
          recordCollectorServerUrl:
              AppConstants.defaultRecordCollectorServerUrl,
        ),
      );
    } catch (e) {
      // Handle persistence error
    }
  }
}
