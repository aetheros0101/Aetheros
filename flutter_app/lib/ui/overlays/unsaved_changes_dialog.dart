import 'package:flutter/material.dart';

import '../design/aether_theme.dart';

enum UnsavedChangesAction { save, discard, cancel }

/// Shared confirmation when closing a dirty editor tab (WB-103 / WB-501).
class UnsavedChangesDialog extends StatelessWidget {
  const UnsavedChangesDialog({
    super.key,
    required this.fileName,
  });

  final String fileName;

  static Future<UnsavedChangesAction?> show(
    BuildContext context, {
    required String fileName,
  }) {
    return showDialog<UnsavedChangesAction>(
      context: context,
      barrierDismissible: false,
      builder: (_) => UnsavedChangesDialog(fileName: fileName),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: AetherSurfaces.popup,
      title: Text(
        'Unsaved changes',
        style: AetherTypography.uiBody.copyWith(
          color: AetherTextColors.primary,
        ),
      ),
      content: Text(
        '“$fileName” has unsaved changes. Close without saving?',
        style: AetherTypography.uiCaption.copyWith(
          color: AetherTextColors.secondary,
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, UnsavedChangesAction.cancel),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, UnsavedChangesAction.save),
          child: const Text('Save'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, UnsavedChangesAction.discard),
          child: Text(
            "Don't Save",
            style: TextStyle(color: AetherStatus.danger),
          ),
        ),
      ],
    );
  }
}
