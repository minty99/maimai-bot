class VersionOption {
  const VersionOption({
    required this.versionIndex,
    required this.versionName,
    required this.songCount,
  });

  final int versionIndex;
  final String versionName;
  final int songCount;

  factory VersionOption.fromJson(Map<String, dynamic> json) {
    return VersionOption(
      versionIndex: (json['version_index'] as num?)?.toInt() ?? -1,
      versionName: json['version_name'] as String? ?? 'Unknown',
      songCount: (json['song_count'] as num?)?.toInt() ?? 0,
    );
  }
}
