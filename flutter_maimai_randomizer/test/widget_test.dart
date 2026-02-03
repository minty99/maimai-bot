// Basic widget test for maimai randomizer app.

import 'package:flutter_maimai_randomizer/features/settings/bloc/settings/settings_cubit.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_maimai_randomizer/main.dart';

void main() {
  testWidgets('App starts and shows song selection screen', (
    WidgetTester tester,
  ) async {
    // Create a settings cubit for testing
    final settingsCubit = SettingsCubit();

    // Build our app and trigger a frame.
    await tester.pumpWidget(MaimaiRandomizerApp(settingsCubit: settingsCubit));

    // Verify that the app bar shows the correct title.
    expect(find.text('maimai Randomizer'), findsOneWidget);

    // Verify that the initial state text is shown.
    expect(find.text('Press RANDOM to start'), findsOneWidget);
  });
}
