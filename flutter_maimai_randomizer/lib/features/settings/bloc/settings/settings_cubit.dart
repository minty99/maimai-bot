import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../../core/constants/app_constants.dart';
import 'settings_state.dart';

/// Cubit for managing application settings.
///
/// Handles backend URL configuration with SharedPreferences persistence.
class SettingsCubit extends Cubit<SettingsState> {
  SettingsCubit()
    : super(const SettingsState(backendUrl: AppConstants.defaultBackendUrl));

  static const String _backendUrlKey = 'backend_url';

  /// Initialize settings from SharedPreferences.
  ///
  /// Call this during app startup to load persisted settings.
  Future<void> initialize() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final savedUrl = prefs.getString(_backendUrlKey);

      if (savedUrl != null && savedUrl.isNotEmpty) {
        emit(SettingsState(backendUrl: savedUrl));
      }
    } catch (e) {
      // If loading fails, keep default state
      // Could emit an error state here if needed
    }
  }

  /// Update the backend URL and persist it.
  ///
  /// [url] - The new backend URL.
  Future<void> updateBackendUrl(String url) async {
    if (url.isEmpty) {
      return;
    }

    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_backendUrlKey, url);
      emit(state.copyWith(backendUrl: url));
    } catch (e) {
      // Handle persistence error
      // Could emit an error state here if needed
    }
  }

  /// Reset backend URL to default.
  Future<void> resetBackendUrl() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.remove(_backendUrlKey);
      emit(const SettingsState(backendUrl: AppConstants.defaultBackendUrl));
    } catch (e) {
      // Handle persistence error
    }
  }
}
