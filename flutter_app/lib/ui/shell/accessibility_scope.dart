import 'package:flutter/material.dart';

import '../design/foundation/accessibility.dart';

/// Provides [AetherAccessibility] resolved from MediaQuery to the subtree.
class AccessibilityScope extends InheritedWidget {
  const AccessibilityScope({
    super.key,
    required this.accessibility,
    required super.child,
  });

  final AetherAccessibility accessibility;

  static AetherAccessibility of(BuildContext context) {
    final scope =
        context.dependOnInheritedWidgetOfExactType<AccessibilityScope>();
    return scope?.accessibility ?? AetherAccessibility.of(context);
  }

  @override
  bool updateShouldNotify(AccessibilityScope oldWidget) =>
      accessibility != oldWidget.accessibility;
}

/// Builds [AccessibilityScope] from current MediaQuery.
class AccessibilityHost extends StatelessWidget {
  const AccessibilityHost({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return AccessibilityScope(
      accessibility: AetherAccessibility.of(context),
      child: child,
    );
  }
}
