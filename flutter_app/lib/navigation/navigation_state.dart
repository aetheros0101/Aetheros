import 'navigation_item.dart';

/// Single source of truth for active activity + registered nav items.
///
/// [activeId] is persisted; [items] are registered at runtime (icons/builders
/// are not serializable).
class NavigationState {
  const NavigationState({
    this.items = const [],
    this.activeId = BuiltInActivities.explorer,
  });

  final List<NavigationItem> items;
  final String activeId;

  NavigationItem? get active {
    for (final i in items) {
      if (i.id == activeId) return i;
    }
    return items.isEmpty ? null : items.first;
  }

  NavigationState copyWith({
    List<NavigationItem>? items,
    String? activeId,
  }) {
    return NavigationState(
      items: items ?? this.items,
      activeId: activeId ?? this.activeId,
    );
  }

  NavigationState activate(String id) {
    if (items.isNotEmpty && !items.any((i) => i.id == id)) return this;
    return copyWith(activeId: id);
  }

  /// Persist only activeId — items are re-registered on boot.
  Map<String, dynamic> toJson() => {'activeId': activeId};

  factory NavigationState.fromJson(Map<String, dynamic> json) {
    return NavigationState(
      activeId: json['activeId'] as String? ?? BuiltInActivities.explorer,
    );
  }
}
