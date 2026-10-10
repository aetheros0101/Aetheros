import 'package:flutter/material.dart';

import '../../editor/editor_document.dart';
import '../design/aether_theme.dart';

enum ConflictResolution { reload, overwrite, keepLocal, cancel }

/// Shown when atomic save detects version mismatch or external modification.
class SaveConflictDialog extends StatelessWidget {
  const SaveConflictDialog({
    super.key,
    required this.conflict,
    this.isExternal = false,
  });

  final SaveConflict conflict;
  final bool isExternal;

  static Future<ConflictResolution?> show(
    BuildContext context, {
    required SaveConflict conflict,
    bool isExternal = false,
  }) {
    return showDialog<ConflictResolution>(
      context: context,
      barrierDismissible: false,
      builder: (_) => SaveConflictDialog(
        conflict: conflict,
        isExternal: isExternal,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final title = isExternal
        ? 'External modification detected'
        : 'Save conflict';
    final body = isExternal
        ? 'The file was changed outside the editor.\n\n${conflict.path}'
        : 'The file changed on disk since you opened it.\n\n'
            '${conflict.path}\n'
            '${conflict.message ?? ''}';

    return AlertDialog(
      backgroundColor: AetherSurfaces.popup,
      title: Text(title, style: AetherTypography.uiTitle.copyWith(
        color: AetherTextColors.primary,
      )),
      content: Text(
        body.trim(),
        style: AetherTypography.uiBody.copyWith(color: AetherTextColors.secondary),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, ConflictResolution.cancel),
          child: Text('Cancel', style: TextStyle(color: AetherTextColors.secondary)),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, ConflictResolution.keepLocal),
          child: Text('Keep Local', style: TextStyle(color: AetherTextColors.secondary)),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, ConflictResolution.reload),
          child: Text('Reload', style: TextStyle(color: AetherAccent.primary)),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, ConflictResolution.overwrite),
          child: const Text('Overwrite'),
        ),
      ],
    );
  }
}
