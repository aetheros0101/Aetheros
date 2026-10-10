import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/navigation/navigation_item.dart';
import 'package:aetheros_app/navigation/navigation_state.dart';
import 'package:aetheros_app/ui/design/foundation/icons.dart';

void main() {
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

  test('activate switches activeId', () {
    final n = NavigationState(items: items, activeId: BuiltInActivities.explorer);
    final next = n.activate(BuiltInActivities.search);
    expect(next.activeId, BuiltInActivities.search);
    expect(next.active?.title, 'Search');
  });

  test('activate unknown id is ignored when items non-empty', () {
    final n = NavigationState(items: items, activeId: BuiltInActivities.explorer);
    final next = n.activate('unknown');
    expect(next.activeId, BuiltInActivities.explorer);
  });

  test('toJson/fromJson persists only activeId', () {
    final n = NavigationState(items: items, activeId: BuiltInActivities.search);
    final restored = NavigationState.fromJson(n.toJson());
    expect(restored.activeId, BuiltInActivities.search);
    expect(restored.items, isEmpty);
  });
}
