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
  // Random Filter Defaults
  // ─────────────────────────────────────────────────────────────────────────

  /// All chart type filters enabled by default.
  static const List<String> defaultEnabledChartTypes = ['STD', 'DX'];

  /// All difficulty filters enabled by default (DifficultyCategory index).
  static const List<int> defaultEnabledDifficultyIndices = [0, 1, 2, 3, 4];

  /// Difficulty labels by DifficultyCategory index.
  static const Map<int, String> difficultyLabelsByIndex = {
    0: 'BASIC',
    1: 'ADVANCED',
    2: 'EXPERT',
    3: 'MASTER',
    4: 'Re:MASTER',
  };

  /// Show display level (e.g., "13+") by default.
  static const bool defaultShowLevel = true;

  /// Show user level label (e.g., "(A)") by default.
  static const bool defaultShowUserLevel = true;

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
