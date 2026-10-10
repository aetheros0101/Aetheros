import 'package:flutter/material.dart';

import '../../state/app_state.dart';
import '../design/aether_theme.dart';
import 'responsive.dart';

/// Title bar: app name + workspace; on mobile, menu / command actions.
class AetherTitleBar extends StatelessWidget {
  const AetherTitleBar({
    super.key,
    required this.state,
    this.height = 36,
    this.onMenu,
    this.onCommandPalette,
    this.onQuickOpen,
  });

  final AppState state;
  final double height;
  final VoidCallback? onMenu;
  final VoidCallback? onCommandPalette;
  final VoidCallback? onQuickOpen;

  @override
  Widget build(BuildContext context) {
    final name = state.workspace.rootName ?? 'AetherOS';
    final width = MediaQuery.sizeOf(context).width;
    final mobile = layoutModeForWidth(width) == AetherLayoutMode.mobile;

    return Container(
      height: height,
      color: AetherSurfaces.chrome,
      padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.sm),
      child: Row(
        children: [
          if (mobile && onMenu != null) ...[
            IconButton(
              icon: const Icon(AetherIcons.menu, size: 18),
              color: AetherTextColors.secondary,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              tooltip: 'Menu',
              onPressed: onMenu,
            ),
            const SizedBox(width: 4),
          ],
          Text(
            'AetherOS',
            style: AetherTypography.uiLabel.copyWith(
              color: AetherTextColors.secondary,
            ),
          ),
          const SizedBox(width: AetherSpacing.sm),
          Text(
            '—',
            style: AetherTypography.uiLabel.copyWith(
              color: AetherTextColors.tertiary,
            ),
          ),
          const SizedBox(width: AetherSpacing.sm),
          Expanded(
            child: Text(
              name,
              style: AetherTypography.uiLabel.copyWith(
                color: AetherTextColors.primary,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          if (mobile) ...[
            if (onQuickOpen != null)
              IconButton(
                icon: const Icon(AetherIcons.file, size: 18),
                color: AetherTextColors.secondary,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
                tooltip: 'Quick Open',
                onPressed: onQuickOpen,
              ),
            if (onCommandPalette != null)
              IconButton(
                icon: const Icon(AetherIcons.search, size: 18),
                color: AetherTextColors.secondary,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
                tooltip: 'Command Palette',
                onPressed: onCommandPalette,
              ),
          ],
        ],
      ),
    );
  }
}
