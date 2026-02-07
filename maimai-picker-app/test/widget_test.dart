// Basic widget test for maimai picker app.

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

    // Build our app and trigger a frame.
    await tester.pumpWidget(MaimaiPickerApp(
      settingsCubit: settingsCubit,
      levelRangeCubit: levelRangeCubit,
    ));

    // Verify that the app bar shows the correct title.
    expect(find.text('maimai picker'), findsOneWidget);

    // Verify that the initial state text is shown.
    expect(find.text('Press RANDOM to start'), findsOneWidget);
  });
}
