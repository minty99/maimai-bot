import 'package:equatable/equatable.dart';

import '../../data/models/song_model.dart';

/// Sealed base class for SongCubit states
sealed class SongState extends Equatable {
  const SongState();

  @override
  List<Object?> get props => [];
}

/// Initial state - no song loaded yet
class SongInitial extends SongState {
  const SongInitial();
}

/// Loading state - fetching a random song
class SongLoading extends SongState {
  const SongLoading();
}

/// Loaded state - song successfully fetched
class SongLoaded extends SongState {
  final SongModel song;

  const SongLoaded(this.song);

  @override
  List<Object?> get props => [song];
}

/// Error state - something went wrong during fetch
class SongError extends SongState {
  final String message;

  const SongError(this.message);

  @override
  List<Object?> get props => [message];
}

/// Not found state - no songs available in the specified range
class SongNotFound extends SongState {
  const SongNotFound();
}
