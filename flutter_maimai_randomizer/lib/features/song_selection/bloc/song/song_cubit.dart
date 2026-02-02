import 'package:flutter_bloc/flutter_bloc.dart';

import '../../data/repositories/song_repository.dart';
import 'song_state.dart';

/// Cubit for managing song fetching and state.
///
/// Handles fetching random songs from the repository and emitting
/// appropriate states (Loading, Loaded, Error, NotFound).
class SongCubit extends Cubit<SongState> {
  SongCubit({required SongRepository repository})
    : _repository = repository,
      super(const SongInitial());

  final SongRepository _repository;

  /// Fetch a random song within the specified level range.
  ///
  /// Emits:
  /// - [SongLoading] immediately
  /// - [SongLoaded] on success
  /// - [SongNotFound] if no songs available in range
  /// - [SongError] on network or other errors
  Future<void> fetchRandomSong({
    required double minLevel,
    required double maxLevel,
  }) async {
    emit(const SongLoading());

    try {
      final song = await _repository.getRandomSong(
        minLevel: minLevel,
        maxLevel: maxLevel,
      );
      emit(SongLoaded(song));
    } on NotFoundException {
      emit(const SongNotFound());
    } on NetworkException catch (e) {
      emit(SongError(e.message));
    } catch (e) {
      emit(SongError('Unexpected error: $e'));
    }
  }
}
