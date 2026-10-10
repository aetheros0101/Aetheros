import 'package:flutter/material.dart';

import '../../application/services/terminal_service.dart';
import '../../state/app_state.dart';
import '../design/aether_theme.dart';
import '../views/terminal/terminal_view.dart';

class AetherBottomPanel extends StatelessWidget {
  const AetherBottomPanel({
    super.key,
    required this.state,
    required this.onStateUpdate,
    required this.terminal,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final TerminalService terminal;

  @override
  Widget build(BuildContext context) {
    final height = state.workbench.clampedBottomPanelHeight;
    final panels = state.panel.panels;
    final activeId = state.panel.activePanelId;

    return SizedBox(
      height: height,
      child: Container(
        color: AetherSurfaces.panel,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Container(
              height: 32,
              color: AetherSurfaces.chrome,
              child: Row(
                children: [
                  for (final p in panels)
                    _PanelTab(
                      label: p.label,
                      selected: p.id == activeId,
                      badge: p.badgeCount,
                      onTap: () => onStateUpdate(
                          (s) => s.copyWith(panel: s.panel.activate(p.id))),
                    ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(AetherIcons.close, size: 16),
                    color: AetherTextColors.tertiary,
                    onPressed: () => onStateUpdate((s) => s.copyWith(
                          workbench: s.workbench
                              .copyWith(bottomPanelVisible: false),
                        )),
                    padding: EdgeInsets.zero,
                    constraints:
                        const BoxConstraints(minWidth: 32, minHeight: 32),
                  ),
                ],
              ),
            ),
            Expanded(child: _body(activeId)),
          ],
        ),
      ),
    );
  }

  Widget _body(String id) {
    switch (id) {
      case 'terminal':
        return TerminalView(state: state, terminal: terminal);
      case 'problems':
        return Padding(
          padding: const EdgeInsets.all(AetherSpacing.sm),
          child: Text(
            state.problems.items.isEmpty
                ? 'No problems detected.'
                : '${state.problems.items.length} problem(s)',
            style: AetherTypography.uiBody.copyWith(
              color: AetherTextColors.secondary,
            ),
          ),
        );
      default:
        return Padding(
          padding: const EdgeInsets.all(AetherSpacing.sm),
          child: Text(
            id,
            style: AetherTypography.uiBody.copyWith(
              color: AetherTextColors.secondary,
            ),
          ),
        );
    }
  }
}

class _PanelTab extends StatelessWidget {
  const _PanelTab({
    required this.label,
    required this.selected,
    required this.onTap,
    this.badge,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;
  final int? badge;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          border: Border(
            bottom: BorderSide(
              color: selected ? AetherAccent.primary : Colors.transparent,
              width: 2,
            ),
          ),
        ),
        child: Row(
          children: [
            Text(
              label,
              style: AetherTypography.uiCaption.copyWith(
                color: selected
                    ? AetherTextColors.primary
                    : AetherTextColors.secondary,
              ),
            ),
            if (badge != null && badge! > 0) ...[
              const SizedBox(width: 4),
              Text(
                '$badge',
                style: AetherTypography.uiMicro.copyWith(
                  color: AetherStatus.danger,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
