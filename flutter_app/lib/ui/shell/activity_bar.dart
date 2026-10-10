import 'package:flutter/material.dart';

import '../../navigation/navigation_item.dart';
import '../../state/app_state.dart';
import '../design/aether_theme.dart';

class AetherActivityBar extends StatelessWidget {
  const AetherActivityBar({
    super.key,
    required this.state,
    required this.onStateUpdate,
    this.width = 48,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final double width;

  @override
  Widget build(BuildContext context) {
    final items = state.navigation.items;
    final activeId = state.navigation.activeId;

    return Container(
      width: width,
      color: AetherSurfaces.chrome,
      child: Column(
        children: [
          for (final item in items)
            _ActivityButton(
              item: item,
              selected: item.id == activeId,
              onTap: () {
                onStateUpdate((s) => s.copyWith(
                      navigation: s.navigation.activate(item.id),
                      workbench: s.workbench.copyWith(
                        primarySidebarVisible: true,
                      ),
                    ));
              },
            ),
          const Spacer(),
          _ActivityButton(
            item: const NavigationItem(
              id: BuiltInActivities.settings,
              title: 'Settings',
              icon: AetherIcons.settings,
            ),
            selected: activeId == BuiltInActivities.settings,
            onTap: () {
              // Settings is not in navigation.items (footer control only).
              // Set activeId directly so PrimarySidebar can show SettingsView.
              onStateUpdate((s) => s.copyWith(
                    navigation: s.navigation.copyWith(
                      activeId: BuiltInActivities.settings,
                    ),
                    workbench: s.workbench.copyWith(
                      primarySidebarVisible: true,
                    ),
                  ));
            },
          ),
        ],
      ),
    );
  }
}

class _ActivityButton extends StatelessWidget {
  const _ActivityButton({
    required this.item,
    required this.selected,
    required this.onTap,
  });

  final NavigationItem item;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final color = selected ? AetherAccent.primary : AetherTextColors.secondary;
    return Tooltip(
      message: item.title,
      child: InkWell(
        onTap: onTap,
        child: Container(
          width: 48,
          height: 48,
          alignment: Alignment.center,
          decoration: selected
              ? const BoxDecoration(
                  border: Border(
                    left: BorderSide(color: AetherAccent.primary, width: 2),
                  ),
                )
              : null,
          child: Icon(item.icon, size: AetherIcons.sizeLg, color: color),
        ),
      ),
    );
  }
}
