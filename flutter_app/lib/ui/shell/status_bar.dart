import 'package:flutter/material.dart';

import '../../state/app_state.dart';
import '../design/aether_theme.dart';

class AetherStatusBar extends StatelessWidget {
  const AetherStatusBar({
    super.key,
    required this.state,
    this.height = 24,
  });

  final AppState state;
  final double height;

  @override
  Widget build(BuildContext context) {
    final git = state.git;
    final problems = state.problems;

    return Container(
      height: height,
      color: AetherSurfaces.chrome,
      padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.sm),
      child: Row(
        children: [
          if (git.branch != null) ...[
            Icon(AetherIcons.branch, size: 12, color: AetherTextColors.secondary),
            const SizedBox(width: 4),
            Text(
              git.branch!,
              style: AetherTypography.uiMicro.copyWith(
                color: AetherTextColors.secondary,
              ),
            ),
            if (git.ahead > 0 || git.behind > 0) ...[
              const SizedBox(width: 6),
              Text(
                '↑${git.ahead} ↓${git.behind}',
                style: AetherTypography.uiMicro.copyWith(
                  color: AetherTextColors.tertiary,
                ),
              ),
            ],
            const SizedBox(width: AetherSpacing.md),
          ],
          if (problems.errorCount > 0 || problems.warningCount > 0) ...[
            Icon(AetherIcons.error, size: 12, color: AetherStatus.danger),
            const SizedBox(width: 2),
            Text('${problems.errorCount}',
                style: AetherTypography.uiMicro.copyWith(color: AetherTextColors.secondary)),
            const SizedBox(width: 6),
            Icon(AetherIcons.warning, size: 12, color: AetherStatus.warning),
            const SizedBox(width: 2),
            Text('${problems.warningCount}',
                style: AetherTypography.uiMicro.copyWith(color: AetherTextColors.secondary)),
          ],
          const Spacer(),
          Text(
            'UTF-8',
            style: AetherTypography.uiMicro.copyWith(color: AetherTextColors.tertiary),
          ),
          const SizedBox(width: AetherSpacing.md),
          Text(
            'LF',
            style: AetherTypography.uiMicro.copyWith(color: AetherTextColors.tertiary),
          ),
          const SizedBox(width: AetherSpacing.md),
          Text(
            state.workbench.density,
            style: AetherTypography.uiMicro.copyWith(color: AetherTextColors.tertiary),
          ),
        ],
      ),
    );
  }
}
