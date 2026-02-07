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
  /// Throws:
  /// - [NotFoundException] if no songs are available in the range
  /// - [NetworkException] if a network error occurs
  Future<SongModel> getRandomSong({
    required double minLevel,
    required double maxLevel,
  });
}

/// Implementation of SongRepository using HTTP requests.
class SongRepositoryImpl implements SongRepository {
  SongRepositoryImpl({required this.baseUrl}) {
    _dio = Dio(
      BaseOptions(
        baseUrl: baseUrl,
        connectTimeout: const Duration(seconds: 10),
        receiveTimeout: const Duration(seconds: 10),
      ),
    );
  }

  final String baseUrl;
  late final Dio _dio;

  @override
  Future<SongModel> getRandomSong({
    required double minLevel,
    required double maxLevel,
  }) async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        '/api/songs/random',
        queryParameters: {'min_level': minLevel, 'max_level': maxLevel},
      );

      if (response.statusCode == 404) {
        throw const NotFoundException('No songs found in the specified range');
      }

      if (response.data == null) {
        throw const NetworkException('Empty response from server');
      }

      return SongModel.fromJson(response.data!, baseUrl);
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
    } catch (e) {
      throw NetworkException('Unexpected error: $e');
    }
  }
}
