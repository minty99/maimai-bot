import 'package:equatable/equatable.dart';

/// State for SettingsCubit
class SettingsState extends Equatable {
  const SettingsState({
    required this.songInfoServerUrl,
    required this.recordCollectorServerUrl,
    required this.enabledChartTypes,
    required this.enabledDifficultyIndices,
    required this.includeVersionIndices,
    required this.showLevel,
    required this.showUserLevel,
  });

  /// Song Info Server URL (song data, covers).
  final String songInfoServerUrl;

  /// Record Collector Server URL (personal scores/playlogs).
  final String recordCollectorServerUrl;

  /// Enabled chart type filters (e.g., STD, DX).
  final Set<String> enabledChartTypes;

  /// Enabled difficulty filters (DifficultyCategory indices).
  final Set<int> enabledDifficultyIndices;

  /// Included version indices for random song selection.
  ///
  /// - `null`: include all versions (do not send include_versions query)
  /// - non-null set: include only those version indices
  final Set<int>? includeVersionIndices;

  /// Whether to show display level text (e.g., "13+") in song card.
  final bool showLevel;

  /// Whether to show user level label (e.g., "(A)") next to internal level.
  final bool showUserLevel;

  static const Object _includeVersionIndicesNoChange = Object();

  /// Creates a copy of this state with optional field overrides.
  SettingsState copyWith({
    String? songInfoServerUrl,
    String? recordCollectorServerUrl,
    Set<String>? enabledChartTypes,
    Set<int>? enabledDifficultyIndices,
    Object? includeVersionIndices = _includeVersionIndicesNoChange,
    bool? showLevel,
    bool? showUserLevel,
  }) {
    return SettingsState(
      songInfoServerUrl: songInfoServerUrl ?? this.songInfoServerUrl,
      recordCollectorServerUrl:
          recordCollectorServerUrl ?? this.recordCollectorServerUrl,
      enabledChartTypes: enabledChartTypes ?? this.enabledChartTypes,
      enabledDifficultyIndices:
          enabledDifficultyIndices ?? this.enabledDifficultyIndices,
      includeVersionIndices:
          identical(includeVersionIndices, _includeVersionIndicesNoChange)
          ? this.includeVersionIndices
          : includeVersionIndices as Set<int>?,
      showLevel: showLevel ?? this.showLevel,
      showUserLevel: showUserLevel ?? this.showUserLevel,
    );
  }

  @override
  List<Object?> get props => [
    songInfoServerUrl,
    recordCollectorServerUrl,
    _sortedStrings(enabledChartTypes).join(','),
    _sortedInts(enabledDifficultyIndices).join(','),
    includeVersionIndices == null
        ? null
        : _sortedInts(includeVersionIndices!).join(','),
    showLevel,
    showUserLevel,
  ];

  static List<String> _sortedStrings(Set<String> values) {
    final sorted = values.toList()..sort();
    return sorted;
  }

  static List<int> _sortedInts(Set<int> values) {
    final sorted = values.toList()..sort();
    return sorted;
  }
}
