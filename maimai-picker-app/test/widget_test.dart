// Basic widget test for maimai picker app.
//
// Note: SongSelectionScreen fires a Dio network call in initState
// (version options fetch). In the test environment this creates a
// pending timer that the framework flags. We pump extra frames to let
// the timer complete/fail before the test tears down.

import 'package:maimai_picker_app/features/settings/bloc/settings/settings_cubit.dart';
import 'package:maimai_picker_app/features/song_selection/bloc/level_range/level_range_cubit.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:maimai_picker_app/main.dart';

void main() {
  testWidgets('App starts and shows song selection screen', (
    WidgetTester tester,
  ) async {
    // Create cubits for testing
    final settingsCubit = SettingsCubit();
    final levelRangeCubit = LevelRangeCubit();

    addTearDown(() async {
      await settingsCubit.close();
      await levelRangeCubit.close();
    });

    // Build our app and trigger a frame.
    await tester.pumpWidget(
      MaimaiPickerApp(
        settingsCubit: settingsCubit,
        levelRangeCubit: levelRangeCubit,
      ),
    );

    // Verify that the initial prompt is shown (AppBar was removed in redesign).
    expect(find.text('Press RANDOM or shake.'), findsOneWidget);

    // Drain pending Dio timer from version-fetch kicked off in initState.
    await tester.pump(const Duration(seconds: 5));
  });
}
