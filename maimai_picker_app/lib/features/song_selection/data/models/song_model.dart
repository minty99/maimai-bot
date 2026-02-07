import 'package:equatable/equatable.dart';

/// Model representing a maimai song with score data.
///
/// Maps to backend's ScoreResponse structure from /api/songs/random endpoint.
class SongModel extends Equatable {
  const SongModel({
    required this.title,
    required this.chartType,
    required this.diffCategory,
    required this.level,
    required this.imageUrl,
    this.achievementX10000,
    this.rank,
    this.fc,
    this.sync,
    this.dxScore,
    this.dxScoreMax,
    this.sourceIdx,
    this.internalLevel,
    this.version,
    this.ratingPoints,
    this.bucket,
  });

  /// Song title
  final String title;

  /// Chart type: "STD" or "DX"
  final String chartType;

  /// Difficulty category: "BASIC", "ADVANCED", "EXPERT", "MASTER", "Re:MASTER"
  final String diffCategory;

  /// Display level (e.g., "12+", "13")
  final String level;

  /// Full jacket image URL
  final String imageUrl;

  /// Achievement score as integer (percent * 10000)
  /// Null if not played yet
  final int? achievementX10000;

  /// Score rank: "SSS+", "SSS", "SS+", etc.
  final String? rank;

  /// FC status: "AP+", "AP", "FC+", "FC"
  final String? fc;

  /// Sync status: "FDX+", "FDX", "FS+", "FS", "SYNC"
  final String? sync;

  /// DX score achieved
  final int? dxScore;

  /// Maximum possible DX score
  final int? dxScoreMax;

  /// Internal source index
  final String? sourceIdx;

  /// Internal level (e.g., 12.5, 13.7)
  final double? internalLevel;

  /// Song version (e.g., "PRiSM PLUS", "CiRCLE")
  final String? version;

  /// Calculated rating points
  final int? ratingPoints;

  /// "New" or "Old" (based on version)
  final String? bucket;

  /// Create SongModel from JSON response.
  factory SongModel.fromJson(Map<String, dynamic> json, String baseUrl) {
    // Construct jacket image URL from image_name
    final imageName = json['image_name'] as String?;
    final imageUrl = imageName != null && imageName.isNotEmpty
        ? '$baseUrl/api/cover/$imageName'
        : '';

    return SongModel(
      title: json['title'] as String,
      chartType: json['chart_type'] as String,
      diffCategory: json['diff_category'] as String,
      level: json['level'] as String,
      imageUrl: imageUrl,
      achievementX10000: json['achievement_x10000'] as int?,
      rank: json['rank'] as String?,
      fc: json['fc'] as String?,
      sync: json['sync'] as String?,
      dxScore: json['dx_score'] as int?,
      dxScoreMax: json['dx_score_max'] as int?,
      sourceIdx: json['source_idx'] as String?,
      internalLevel: (json['internal_level'] as num?)?.toDouble(),
      version: json['version'] as String?,
      ratingPoints: json['rating_points'] as int?,
      bucket: json['bucket'] as String?,
    );
  }

  @override
  List<Object?> get props => [
    title,
    chartType,
    diffCategory,
    level,
    imageUrl,
    achievementX10000,
    rank,
    fc,
    sync,
    dxScore,
    dxScoreMax,
    sourceIdx,
    internalLevel,
    version,
    ratingPoints,
    bucket,
  ];
}
