import 'dart:math';

import 'package:dio/dio.dart';

import '../models/song_model.dart';

/// Exception thrown when no songs are found in the specified range.
class NotFoundException implements Exception {
  final String message;
  const NotFoundException(this.message);

  @override
  String toString() => 'NotFoundException: $message';
}

/// Exception thrown when a network error occurs.
class NetworkException implements Exception {
  final String message;
  const NetworkException(this.message);

  @override
  String toString() => 'NetworkException: $message';
}

/// Abstract repository for fetching song data.
abstract class SongRepository {
  /// Fetch a random song within the specified internal level range.
  ///
  /// Returns a [SongModel] with song metadata from the Song Info Server.
  /// If a Record Collector Server URL is configured, personal achievement
  /// data is fetched and merged into the model. If the record collector
  /// is unreachable or not configured, the model is returned without
  /// personal data (degraded mode).
  ///
  /// Throws:
  /// - [NotFoundException] if no songs are available in the range
  /// - [NetworkException] if a network error occurs with the Song Info Server
  Future<SongModel> getRandomSong({
    required double minLevel,
    required double maxLevel,
  });
}

/// Implementation of SongRepository using HTTP requests.
///
/// Connects to two backend servers:
/// - **Song Info Server** (required): provides song metadata and random selection
/// - **Record Collector Server** (optional): provides personal achievement data
class SongRepositoryImpl implements SongRepository {
  SongRepositoryImpl({
    required this.songInfoServerUrl,
    this.recordCollectorServerUrl,
  }) {
    _songInfoDio = Dio(
      BaseOptions(
        baseUrl: songInfoServerUrl,
        connectTimeout: const Duration(seconds: 10),
        receiveTimeout: const Duration(seconds: 10),
      ),
    );

    final rcUrl = recordCollectorServerUrl;
    if (rcUrl != null && rcUrl.trim().isNotEmpty) {
      _recordCollectorDio = Dio(
        BaseOptions(
          baseUrl: rcUrl,
          connectTimeout: const Duration(seconds: 5),
          receiveTimeout: const Duration(seconds: 5),
        ),
      );
    }
  }

  final String songInfoServerUrl;
  final String? recordCollectorServerUrl;
  late final Dio _songInfoDio;
  Dio? _recordCollectorDio;

  static final _random = Random();

  @override
  Future<SongModel> getRandomSong({
    required double minLevel,
    required double maxLevel,
  }) async {
    // 1. Fetch a random song from Song Info Server
    final songResponse = await _fetchRandomSongFromSongInfo(
      minLevel: minLevel,
      maxLevel: maxLevel,
    );

    // Parse the nested response: pick one random sheet within the level range
    final title = songResponse['title'] as String;
    final version = songResponse['version'] as String?;
    final imageName = songResponse['image_name'] as String?;
    final sheets = songResponse['sheets'] as List<dynamic>? ?? [];

    if (sheets.isEmpty) {
      throw const NotFoundException('No sheets found in the selected song');
    }

    // Filter sheets that match the level range, then pick one at random
    final matchingSheets = sheets.where((sheet) {
      final il = (sheet as Map<String, dynamic>)['internal_level'];
      if (il == null) return false;
      final level = (il as num).toDouble();
      return level >= minLevel && level <= maxLevel;
    }).toList();

    final pool = matchingSheets.isNotEmpty ? matchingSheets : sheets;
    final selectedSheet =
        pool[_random.nextInt(pool.length)] as Map<String, dynamic>;

    final chartType = selectedSheet['chart_type'] as String? ?? 'STD';
    final diffCategory = selectedSheet['difficulty'] as String? ?? '';
    final level = selectedSheet['level'] as String? ?? '';
    final internalLevel = (selectedSheet['internal_level'] as num?)?.toDouble();

    final imageUrl = (imageName != null && imageName.isNotEmpty)
        ? '$songInfoServerUrl/api/cover/$imageName'
        : '';

    // 2. Optionally fetch personal score from Record Collector Server
    Map<String, dynamic>? personalScore;
    if (_recordCollectorDio != null) {
      personalScore = await _fetchPersonalScore(
        title: title,
        chartType: chartType,
        diffCategory: diffCategory,
      );
    }

    return SongModel(
      title: title,
      chartType: chartType,
      diffCategory: diffCategory,
      level: level,
      imageUrl: imageUrl,
      internalLevel: internalLevel,
      version: version,
      // Personal data from record-collector-server (null if not available)
      achievementX10000: personalScore?['achievement_x10000'] as int?,
      rank: personalScore?['rank'] as String?,
      fc: personalScore?['fc'] as String?,
      sync: personalScore?['sync'] as String?,
      dxScore: personalScore?['dx_score'] as int?,
      dxScoreMax: personalScore?['dx_score_max'] as int?,
      sourceIdx: personalScore?['source_idx'] as String?,
      ratingPoints: personalScore?['rating_points'] as int?,
      bucket: personalScore?['bucket'] as String?,
    );
  }

  /// Fetch a random song from the Song Info Server.
  Future<Map<String, dynamic>> _fetchRandomSongFromSongInfo({
    required double minLevel,
    required double maxLevel,
  }) async {
    try {
      final response = await _songInfoDio.get<Map<String, dynamic>>(
        '/api/songs/random',
        queryParameters: {'min_level': minLevel, 'max_level': maxLevel},
      );

      if (response.statusCode == 404) {
        throw const NotFoundException('No songs found in the specified range');
      }

      if (response.data == null) {
        throw const NetworkException('Empty response from server');
      }

      return response.data!;
    } on DioException catch (e) {
      if (e.response?.statusCode == 404) {
        throw const NotFoundException('No songs found in the specified range');
      }

      if (e.type == DioExceptionType.connectionTimeout ||
          e.type == DioExceptionType.receiveTimeout) {
        throw const NetworkException('Request timeout');
      }

      if (e.type == DioExceptionType.connectionError) {
        throw NetworkException(
          'Connection failed: ${e.message ?? "unknown error"}',
        );
      }

      throw NetworkException('Network error: ${e.message ?? "unknown error"}');
    }
  }

  /// Optionally fetch personal score from Record Collector Server.
  ///
  /// Returns null if the server is unreachable or the score is not found.
  /// Failures are silently ignored (degraded mode).
  Future<Map<String, dynamic>?> _fetchPersonalScore({
    required String title,
    required String chartType,
    required String diffCategory,
  }) async {
    try {
      final encodedTitle = Uri.encodeComponent(title);
      final encodedChartType = Uri.encodeComponent(chartType);
      final encodedDiffCategory = Uri.encodeComponent(diffCategory);

      final response = await _recordCollectorDio!.get<Map<String, dynamic>>(
        '/api/scores/$encodedTitle/$encodedChartType/$encodedDiffCategory',
      );

      if (response.statusCode == 200 && response.data != null) {
        return response.data;
      }
    } on DioException {
      // Silently ignore - record collector is optional
    } catch (_) {
      // Silently ignore - record collector is optional
    }
    return null;
  }
}
