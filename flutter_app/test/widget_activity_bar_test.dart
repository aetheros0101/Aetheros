import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/navigation/navigation_item.dart';
import 'package:aetheros_app/navigation/navigation_state.dart';
import 'package:aetheros_app/state/app_state.dart';
import 'package:aetheros_app/ui/design/foundation/icons.dart';
import 'package:aetheros_app/ui/shell/activity_bar.dart';

void main() {
  testWidgets('Activity Bar selects explorer and shows settings', (tester) async {
    final items = [
      NavigationItem(
        id: BuiltInActivities.explorer,
        title: 'Explorer',
        icon: AetherIcons.files,
        order: 10,
      ),
      NavigationItem(
        id: BuiltInActivities.search,
        title: 'Search',
        icon: AetherIcons.search,
        order: 20,
      ),
    ];
    var state = AppState(
      navigation: NavigationState(
        items: items,
        activeId: BuiltInActivities.explorer,
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              AetherActivityBar(
                state: state,
                onStateUpdate: (fn) {
                  state = fn(state);
                },
              ),
            ],
          ),
        ),
      ),
    );

    expect(find.byIcon(AetherIcons.files), findsOneWidget);
    expect(find.byIcon(AetherIcons.search), findsOneWidget);
    expect(find.byIcon(AetherIcons.settings), findsOneWidget);

    await tester.tap(find.byIcon(AetherIcons.search));
    await tester.pump();
    expect(state.navigation.activeId, BuiltInActivities.search);
    expect(state.workbench.primarySidebarVisible, isTrue);
  });
}
