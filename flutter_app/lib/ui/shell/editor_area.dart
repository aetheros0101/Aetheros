import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../editor/editor_controller.dart';
import '../../editor/editor_session.dart';
import '../../state/app_state.dart';
import '../../state/editor_state.dart';
import '../design/aether_theme.dart';
import '../overlays/unsaved_changes_dialog.dart';

class AetherEditorArea extends StatelessWidget {
  const AetherEditorArea({
    super.key,
    required this.state,
    required this.onStateUpdate,
    required this.editorSession,
    this.onSave,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final EditorSession editorSession;
  final Future<void> Function(String path)? onSave;

  @override
  Widget build(BuildContext context) {
    final groups = state.editor.groups;
    if (groups.length <= 1) {
      return _EditorGroupView(
        group: state.editor.activeGroup,
        groupIndex: state.editor.activeGroupIndex,
        state: state,
        onStateUpdate: onStateUpdate,
        editorSession: editorSession,
        onSave: onSave,
      );
    }
    return Row(
      children: [
        for (var i = 0; i < groups.length; i++) ...[
          if (i > 0) Container(width: 1, color: AetherSurfaces.chrome),
          Expanded(
            child: _EditorGroupView(
              group: groups[i],
              groupIndex: i,
              state: state,
              onStateUpdate: onStateUpdate,
              editorSession: editorSession,
              onSave: onSave,
            ),
          ),
        ],
      ],
    );
  }
}

class _EditorGroupView extends StatelessWidget {
  const _EditorGroupView({
    required this.group,
    required this.groupIndex,
    required this.state,
    required this.onStateUpdate,
    required this.editorSession,
    this.onSave,
  });

  final EditorGroupState group;
  final int groupIndex;
  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final EditorSession editorSession;
  final Future<void> Function(String path)? onSave;

  @override
  Widget build(BuildContext context) {
    final tab = group.activeTab;
    final controller =
        tab == null ? null : editorSession.controllerFor(tab.path);

    return Column(
      children: [
        _TabStrip(
          group: group,
          groupIndex: groupIndex,
          activeGroupIndex: state.editor.activeGroupIndex,
          editorSession: editorSession,
          onSelect: (tabIndex) {
            onStateUpdate((s) {
              final g =
                  s.editor.groups[groupIndex].copyWith(activeTabIndex: tabIndex);
              final groups = [...s.editor.groups];
              groups[groupIndex] = g;
              return s.copyWith(
                editor: s.editor.copyWith(
                  groups: groups,
                  activeGroupIndex: groupIndex,
                ),
              );
            });
          },
          onClose: (tabIndex) async {
            final path = group.tabs[tabIndex].path;
            final dirty =
                editorSession.isDirty(path) || group.tabs[tabIndex].isDirty;
            if (dirty) {
              final action = await UnsavedChangesDialog.show(
                context,
                fileName: group.tabs[tabIndex].fileName,
              );
              if (action == null || action == UnsavedChangesAction.cancel) {
                return;
              }
              if (action == UnsavedChangesAction.save) {
                await onSave?.call(path);
                // If still dirty after save attempt, do not close.
                if (editorSession.isDirty(path)) return;
              }
            }
            onStateUpdate((s) {
              final next = s.editor.closeTab(groupIndex, tabIndex);
              editorSession.closeIfUnused(path, next.isPathOpen(path));
              return s.copyWith(editor: next);
            });
          },
        ),
        Expanded(
          child: controller == null
              ? Container(
                  color: AetherSurfaces.editor,
                  alignment: Alignment.center,
                  child: Text(
                    'Open a file from the Explorer',
                    style: AetherTypography.uiBody.copyWith(
                      color: AetherTextColors.tertiary,
                    ),
                  ),
                )
              : _EditorSurface(
                  key: ValueKey(controller.path),
                  controller: controller,
                  onSave: onSave,
                  onDirtyChanged: () {
                    // Rebuild tab strip dirty indicators.
                    onStateUpdate((s) => s);
                  },
                ),
        ),
      ],
    );
  }
}

class _TabStrip extends StatelessWidget {
  const _TabStrip({
    required this.group,
    required this.groupIndex,
    required this.activeGroupIndex,
    required this.editorSession,
    required this.onSelect,
    required this.onClose,
  });

  final EditorGroupState group;
  final int groupIndex;
  final int activeGroupIndex;
  final EditorSession editorSession;
  final ValueChanged<int> onSelect;
  final Future<void> Function(int tabIndex) onClose;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 34,
      color: AetherSurfaces.tab,
      child: ListView.builder(
        scrollDirection: Axis.horizontal,
        itemCount: group.tabs.length,
        itemBuilder: (context, index) {
          final tab = group.tabs[index];
          final selected =
              index == group.activeTabIndex && groupIndex == activeGroupIndex;
          final dirty = editorSession.isDirty(tab.path) || tab.isDirty;
          return InkWell(
            onTap: () => onSelect(index),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              color: selected ? AetherSurfaces.tabActive : null,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (dirty)
                    Container(
                      width: 6,
                      height: 6,
                      margin: const EdgeInsets.only(right: 6),
                      decoration: const BoxDecoration(
                        color: AetherAccent.primary,
                        shape: BoxShape.circle,
                      ),
                    ),
                  Text(
                    dirty ? '${tab.fileName} •' : tab.fileName,
                    style: AetherTypography.fileName.copyWith(
                      color: selected
                          ? AetherTextColors.primary
                          : AetherTextColors.secondary,
                    ),
                  ),
                  const SizedBox(width: 8),
                  InkWell(
                    onTap: () => onClose(index),
                    child: Icon(
                      AetherIcons.close,
                      size: 14,
                      color: AetherTextColors.tertiary,
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}

class _EditorSurface extends StatefulWidget {
  const _EditorSurface({
    super.key,
    required this.controller,
    this.onSave,
    this.onDirtyChanged,
  });

  final EditorController controller;
  final Future<void> Function(String path)? onSave;
  final VoidCallback? onDirtyChanged;

  @override
  State<_EditorSurface> createState() => _EditorSurfaceState();
}

class _EditorSurfaceState extends State<_EditorSurface> {
  late TextEditingController _text;
  bool _syncing = false;

  @override
  void initState() {
    super.initState();
    _text = TextEditingController(text: widget.controller.document.content);
  }

  @override
  void didUpdateWidget(covariant _EditorSurface oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller.path != widget.controller.path) {
      _syncFromDocument();
    } else if (!widget.controller.isDirty &&
        _text.text != widget.controller.document.content) {
      // External reload / save cleaned dirty
      _syncFromDocument();
    }
  }

  void _syncFromDocument() {
    _syncing = true;
    _text.text = widget.controller.document.content;
    _syncing = false;
  }

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  void _onChanged(String value) {
    if (_syncing) return;
    // All mutations go through EditorController — never document.content = …
    widget.controller.setContent(value);
    setState(() {});
    widget.onDirtyChanged?.call();
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyS, control: true): () {
          widget.onSave?.call(widget.controller.path);
        },
        const SingleActivator(LogicalKeyboardKey.keyS, meta: true): () {
          widget.onSave?.call(widget.controller.path);
        },
        const SingleActivator(LogicalKeyboardKey.keyZ, control: true): () {
          if (widget.controller.undo()) {
            _syncFromDocument();
            setState(() {});
          }
        },
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true): () {
          if (widget.controller.undo()) {
            _syncFromDocument();
            setState(() {});
          }
        },
        const SingleActivator(LogicalKeyboardKey.keyZ, control: true, shift: true):
            () {
          if (widget.controller.redo()) {
            _syncFromDocument();
            setState(() {});
          }
        },
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true, shift: true):
            () {
          if (widget.controller.redo()) {
            _syncFromDocument();
            setState(() {});
          }
        },
      },
      child: Focus(
        child: Container(
          color: AetherSurfaces.editor,
          padding: const EdgeInsets.all(AetherSpacing.md),
          child: TextField(
            controller: _text,
            maxLines: null,
            expands: true,
            style:
                AetherTypography.code.copyWith(color: AetherTextColors.primary),
            cursorColor: AetherEditor.cursor,
            decoration: const InputDecoration(
              border: InputBorder.none,
              isCollapsed: true,
            ),
            onChanged: _onChanged,
          ),
        ),
      ),
    );
  }
}


