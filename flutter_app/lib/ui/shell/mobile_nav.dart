import 'package:flutter/material.dart';

import '../../navigation/navigation_item.dart';
import '../../state/app_state.dart';
import '../design/aether_theme.dart';

/// Bottom navigation for mobile layout (WB-301).
///
/// Replaces Activity Bar when width is compact. Selecting an item opens the
/// corresponding primary sidebar view as a full-height drawer overlay.
class AetherMobileBottomNav extends StatelessWidget {
  const AetherMobileBottomNav({
    super.key,
    required this.state,
    required this.onSelect,
    this.onOpenAgent,
    this.height = 56,
  });

  final AppState state;
  final void Function(String activityId) onSelect;
  final VoidCallback? onOpenAgent;
  final double height;

  /// Core activities shown on mobile (Settings included; Run/Debug via palette).
  static const _coreIds = [
    BuiltInActivities.explorer,
    BuiltInActivities.search,
    BuiltInActivities.scm,
    BuiltInActivities.settings,
  ];

  @override
  Widget build(BuildContext context) {
    final fromNav = {
      for (final i in state.navigation.items) i.id: i,
    };

    final items = <NavigationItem>[
      for (final id in _coreIds)
        fromNav[id] ??
            NavigationItem(
              id: id,
              title: _title(id),
              icon: _icon(id),
            ),
    ];

    final active = state.navigation.activeId;
    final drawerOpen = state.workbench.primarySidebarVisible;

    return Container(
      height: height + MediaQuery.paddingOf(context).bottom,
      padding: EdgeInsets.only(bottom: MediaQuery.paddingOf(context).bottom),
      decoration: BoxDecoration(
        color: AetherSurfaces.chrome,
        border: Border(
          top: BorderSide(color: AetherBorders.subtle.color, width: 1),
        ),
      ),
      child: Row(
        children: [
          for (final item in items)
            Expanded(
              child: _NavItem(
                item: item,
                selected: drawerOpen && item.id == active,
                onTap: () => onSelect(item.id),
              ),
            ),
          Expanded(
            child: _NavItem(
              item: const NavigationItem(
                id: BuiltInActivities.agent,
                title: 'Agent',
                icon: AetherIcons.agent,
              ),
              selected: state.workbench.secondarySidebarVisible,
              onTap: () => onOpenAgent?.call(),
            ),
          ),
        ],
      ),
    );
  }

  static String _title(String id) => switch (id) {
        BuiltInActivities.search => 'Search',
        BuiltInActivities.scm => 'Git',
        BuiltInActivities.settings => 'Settings',
        _ => 'Files',
      };

  static IconData _icon(String id) => switch (id) {
        BuiltInActivities.search => AetherIcons.search,
        BuiltInActivities.scm => AetherIcons.branch,
        BuiltInActivities.settings => AetherIcons.settings,
        _ => AetherIcons.files,
      };
}

class _NavItem extends StatelessWidget {
  const _NavItem({
    required this.item,
    required this.selected,
    required this.onTap,
  });

  final NavigationItem item;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final color =
        selected ? AetherAccent.primary : AetherTextColors.secondary;
    return InkWell(
      onTap: onTap,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(item.icon, size: 22, color: color),
          const SizedBox(height: 2),
          Text(
            item.title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: AetherTypography.uiMicro.copyWith(color: color, fontSize: 10),
          ),
        ],
      ),
    );
  }
}
