import 'package:dio/dio.dart';

import '../models/version_option.dart';

abstract class VersionRepository {
  Future<List<VersionOption>> fetchVersionOptions({required String baseUrl});
}

class VersionRepositoryImpl implements VersionRepository {
  VersionRepositoryImpl({Dio? dio}) : _dio = dio ?? Dio();

  final Dio _dio;

  @override
  Future<List<VersionOption>> fetchVersionOptions({
    required String baseUrl,
  }) async {
    var trimmed = baseUrl.trim();
    if (trimmed.endsWith('/')) {
      trimmed = trimmed.substring(0, trimmed.length - 1);
    }
    if (trimmed.isEmpty) {
      return const [];
    }

    try {
      final response = await _dio.get<Map<String, dynamic>>(
        '$trimmed/api/songs/versions',
        options: Options(
          sendTimeout: const Duration(seconds: 5),
          receiveTimeout: const Duration(seconds: 5),
        ),
      );
      final rawVersions =
          response.data?['versions'] as List<dynamic>? ?? const [];
      final versions =
          rawVersions
              .map((raw) => VersionOption.fromJson(raw as Map<String, dynamic>))
              .where((item) => item.versionIndex >= 0)
              .toList()
            ..sort((a, b) => a.versionIndex.compareTo(b.versionIndex));
      return versions;
    } catch (_) {
      return const [];
    }
  }
}
