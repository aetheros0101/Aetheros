import 'package:flutter/material.dart';

import '../../design/aether_theme.dart';

/// Run and Debug activity — not yet backed by a runtime API.
///
/// Explicitly non-functional placeholder so the activity is not mistaken
/// for a completed feature (WB-106).
class RunDebugView extends StatelessWidget {
  const RunDebugView({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(AetherSpacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Run and Debug',
            style: AetherTypography.uiBody.copyWith(
              color: AetherTextColors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: AetherSpacing.sm),
          Text(
            'This view is not implemented yet.\n\n'
            'When the Rust runtime exposes launch / debug session APIs, '
            'start, stop, restart, and output will be wired here.\n\n'
            'No fake run buttons are shown.',
            style: AetherTypography.uiCaption.copyWith(
              color: AetherTextColors.tertiary,
            ),
          ),
        ],
      ),
    );
  }
}
