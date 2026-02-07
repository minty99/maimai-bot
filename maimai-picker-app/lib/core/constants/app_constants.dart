/// Application-wide constants for maimai randomizer.
class AppConstants {
  AppConstants._();

  // ─────────────────────────────────────────────────────────────────────────
  // Server Configuration
  // ─────────────────────────────────────────────────────────────────────────

  /// Default Song Info Server URL (song data, covers).
  static const String defaultSongInfoServerUrl = 'http://localhost:3001';

  /// Default Record Collector Server URL (personal scores/playlogs).
  static const String defaultRecordCollectorServerUrl = 'http://localhost:3000';

  // ─────────────────────────────────────────────────────────────────────────
  // Level Range Configuration
  // ─────────────────────────────────────────────────────────────────────────

  /// Minimum level bound (lowest possible level in maimai).
  static const double minLevelBound = 1.0;

  /// Maximum level bound (highest possible level in maimai).
  static const double maxLevelBound = 15.0;

  /// Default minimum level for song selection.
  static const double defaultMinLevel = 12.5;

  /// Default maximum level for song selection (same as min for gap=0).
  static const double defaultMaxLevel = 12.5;

  /// Default step size when adjusting level range.
  static const double defaultLevelStep = 0.1;

  // ─────────────────────────────────────────────────────────────────────────
  // UI Configuration
  // ─────────────────────────────────────────────────────────────────────────

  /// Minimum touch target size for glove-friendly interaction.
  static const double minTouchTargetSize = 64.0;

  /// Large button height for primary actions.
  static const double largeButtonHeight = 72.0;

  /// Standard padding for screen edges.
  static const double screenPadding = 24.0;
}
