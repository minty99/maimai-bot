import 'package:equatable/equatable.dart';

/// State for SettingsCubit
class SettingsState extends Equatable {
  const SettingsState({
    required this.songInfoServerUrl,
    required this.recordCollectorServerUrl,
  });

  /// Song Info Server URL (song data, covers).
  final String songInfoServerUrl;

  /// Record Collector Server URL (personal scores/playlogs).
  final String recordCollectorServerUrl;

  /// Creates a copy of this state with optional field overrides.
  SettingsState copyWith({
    String? songInfoServerUrl,
    String? recordCollectorServerUrl,
  }) {
    return SettingsState(
      songInfoServerUrl: songInfoServerUrl ?? this.songInfoServerUrl,
      recordCollectorServerUrl:
          recordCollectorServerUrl ?? this.recordCollectorServerUrl,
    );
  }

  @override
  List<Object?> get props => [songInfoServerUrl, recordCollectorServerUrl];
}
