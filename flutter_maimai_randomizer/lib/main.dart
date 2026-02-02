import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'core/theme/app_theme.dart';
import 'features/settings/bloc/settings/settings_cubit.dart';
import 'features/settings/presentation/screens/settings_screen.dart';
import 'features/song_selection/bloc/hardware_input/hardware_input_cubit.dart';
import 'features/song_selection/bloc/level_range/level_range_cubit.dart';
import 'features/song_selection/bloc/song/song_cubit.dart';
import 'features/song_selection/data/repositories/song_repository.dart';
import 'features/song_selection/presentation/screens/song_selection_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Create and initialize settings cubit first (for persisted backend URL)
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
        // Settings Cubit (pre-initialized with persisted URL)
        BlocProvider<SettingsCubit>.value(value: settingsCubit),

        // Level Range Cubit
        BlocProvider<LevelRangeCubit>(create: (_) => LevelRangeCubit()),

        // Hardware Input Cubit
        BlocProvider<HardwareInputCubit>(create: (_) => HardwareInputCubit()),

        // Song Cubit (depends on SettingsCubit for backend URL)
        BlocProvider<SongCubit>(
          create: (context) {
            final backendUrl = context.read<SettingsCubit>().state.backendUrl;
            return SongCubit(
              repository: SongRepositoryImpl(baseUrl: backendUrl),
            );
          },
        ),
      ],
      // Listen to settings changes to recreate SongCubit with new URL
      child: BlocListener<SettingsCubit, dynamic>(
        listenWhen: (previous, current) {
          // Only trigger when backend URL changes
          return previous.backendUrl != current.backendUrl;
        },
        listener: (context, state) {
          // Note: In production, you might want to use a more sophisticated
          // approach like a repository provider pattern. For now, the SongCubit
          // will use the URL it was created with until app restart.
        },
        child: MaterialApp(
          title: 'maimai Randomizer',
          debugShowCheckedModeBanner: false,

          // Material 3 dark theme optimized for arcade use
          theme: AppTheme.darkTheme,
          darkTheme: AppTheme.darkTheme,
          themeMode: ThemeMode.dark,

          // Routes
          initialRoute: SongSelectionScreen.routeName,
          routes: {
            SongSelectionScreen.routeName: (_) => const SongSelectionScreen(),
            SettingsScreen.routeName: (_) => const SettingsScreen(),
          },
        ),
      ),
    );
  }
}
