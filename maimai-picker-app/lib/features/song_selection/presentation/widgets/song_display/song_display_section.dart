import 'package:flutter/material.dart';

import '../../../../../core/theme/app_motion.dart';
import '../../../bloc/song/song_state.dart';
import 'song_error_view.dart';
import 'song_hero_card.dart';
import 'song_initial_view.dart';
import 'song_loading_view.dart';
import 'song_not_found_view.dart';

class SongDisplaySection extends StatelessWidget {
  const SongDisplaySection({
    super.key,
    required this.state,
    required this.showLevel,
    required this.showUserLevel,
  });

  final SongState state;
  final bool showLevel;
  final bool showUserLevel;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: AppMotion.normal,
      switchInCurve: AppMotion.enter,
      switchOutCurve: AppMotion.exit,
      child: switch (state) {
        SongInitial() => const SongInitialView(key: ValueKey('initial')),
        SongLoading() => const SongLoadingView(key: ValueKey('loading')),
        SongNotFound() => const SongNotFoundView(key: ValueKey('not_found')),
        SongError(:final message) => SongErrorView(
          key: const ValueKey('error'),
          message: message,
        ),
        SongLoaded(:final song) => SongHeroCard(
          key: ValueKey(song.hashCode),
          song: song,
          showLevel: showLevel,
          showUserLevel: showUserLevel,
        ),
      },
    );
  }
}
