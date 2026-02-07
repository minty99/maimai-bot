import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'core/theme/app_theme.dart';
import 'features/settings/bloc/settings/settings_cubit.dart';
import 'features/settings/bloc/settings/settings_state.dart';
import 'features/settings/presentation/screens/settings_screen.dart';
import 'features/song_selection/bloc/hardware_input/hardware_input_cubit.dart';
import 'features/song_selection/bloc/level_range/level_range_cubit.dart';
import 'features/song_selection/bloc/song/song_cubit.dart';
import 'features/song_selection/presentation/screens/song_selection_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Create and initialize settings cubit first (for persisted server URLs)
  final settingsCubit = SettingsCubit();
  await settingsCubit.initialize();

  runApp(MaimaiRandomizerApp(settingsCubit: settingsCubit));
}

/// Root application widget with BLoC providers.
class MaimaiRandomizerApp extends StatelessWidget {
  const MaimaiRandomizerApp({super.key, required this.settingsCubit});

  final SettingsCubit settingsCubit;

  @override
  Widget build(BuildContext context) {
    return MultiBlocProvider(
      providers: [
        // Settings Cubit (pre-initialized with persisted server URLs)
        BlocProvider<SettingsCubit>.value(value: settingsCubit),

        // Level Range Cubit
        BlocProvider<LevelRangeCubit>(create: (_) => LevelRangeCubit()),

        // Hardware Input Cubit
        BlocProvider<HardwareInputCubit>(create: (_) => HardwareInputCubit()),
      ],
      // Recreate SongCubit whenever server URLs change
      child: BlocBuilder<SettingsCubit, SettingsState>(
        buildWhen: (previous, current) {
          return previous.songInfoServerUrl != current.songInfoServerUrl ||
              previous.recordCollectorServerUrl !=
                  current.recordCollectorServerUrl;
        },
        builder: (context, state) {
          return BlocProvider<SongCubit>(
            key: ValueKey(
              '${state.songInfoServerUrl}_${state.recordCollectorServerUrl}',
            ),
            create: (_) =>
                SongCubit(songInfoServerUrl: state.songInfoServerUrl),
            child: MaterialApp(
              title: 'maimai picker',
              debugShowCheckedModeBanner: false,

              // Material 3 dark theme optimized for arcade use
              theme: AppTheme.darkTheme,
              darkTheme: AppTheme.darkTheme,
              themeMode: ThemeMode.dark,

              // Routes
              initialRoute: SongSelectionScreen.routeName,
              routes: {
                SongSelectionScreen.routeName: (_) =>
                    const SongSelectionScreen(),
                SettingsScreen.routeName: (_) => const SettingsScreen(),
              },
            ),
          );
        },
      ),
    );
  }
}
