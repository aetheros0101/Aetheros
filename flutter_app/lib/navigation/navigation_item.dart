import 'package:flutter/widgets.dart';

/// Declarative activity-bar / sidebar navigation item.
class NavigationItem {
  const NavigationItem({
    required this.id,
    required this.title,
    required this.icon,
    this.order = 100,
    this.badge,
    this.sidebarBuilder,
  });

  final String id;
  final String title;
  final IconData icon;
  final int order;
  final int? badge;
  final WidgetBuilder? sidebarBuilder;
}

abstract final class BuiltInActivities {
  static const explorer = 'explorer';
  static const search = 'search';
  static const scm = 'scm';
  static const runDebug = 'runDebug';
  static const extensions = 'extensions';
  static const agent = 'agent';
  static const settings = 'settings';
}
