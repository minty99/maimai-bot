import 'package:equatable/equatable.dart';

/// Model representing a maimai song with optional personal score data.
///
/// Song metadata comes from the Song Info Server (/api/songs/random).
/// Personal achievement data (achievement, rank, FC, sync) comes from the
/// Record Collector Server (optional - null when not configured or unreachable).
class SongModel extends Equatable {
  const SongModel({
    required this.title,
    required this.chartType,
    required this.diffCategory,
    required this.level,
    required this.imageUrl,
    this.userLevel,
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
    this.levelSongCount,
    this.filteredSongCount,
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

  /// User-assigned level label (e.g., "12+", "13")
  final String? userLevel;

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

  /// Candidate song count in selected level range before filters.
  final int? levelSongCount;

  /// Candidate song count after applying filters.
  final int? filteredSongCount;

  @override
  List<Object?> get props => [
    title,
    chartType,
    diffCategory,
    level,
    imageUrl,
    userLevel,
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
    levelSongCount,
    filteredSongCount,
  ];
}
