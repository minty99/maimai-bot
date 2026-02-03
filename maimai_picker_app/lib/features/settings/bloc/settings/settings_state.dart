import 'package:equatable/equatable.dart';

/// State for SettingsCubit
class SettingsState extends Equatable {
  const SettingsState({required this.backendUrl});

  /// Backend API URL
  final String backendUrl;

  /// Creates a copy of this state with optional field overrides.
  SettingsState copyWith({String? backendUrl}) {
    return SettingsState(backendUrl: backendUrl ?? this.backendUrl);
  }

  @override
  List<Object?> get props => [backendUrl];
}
