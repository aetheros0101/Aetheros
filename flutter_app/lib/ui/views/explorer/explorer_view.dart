import 'package:flutter/material.dart';

import '../../../state/app_state.dart';
import '../../../state/workspace_state.dart';
import '../../design/aether_theme.dart';

class ExplorerView extends StatelessWidget {
  const ExplorerView({
    super.key,
    required this.state,
    required this.onStateUpdate,
    required this.onOpenFile,
    this.onToggleFolder,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final Future<void> Function(String path) onOpenFile;
  final Future<void> Function(String path)? onToggleFolder;

  @override
  Widget build(BuildContext context) {
    final ws = state.workspace;
    if (ws.rootPath == null) {
      return Center(
        child: Text(
          'No folder open',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.tertiary,
          ),
        ),
      );
    }

    return ListView(
      padding: const EdgeInsets.symmetric(vertical: AetherSpacing.xs),
      children: [
        for (final entry in ws.entries)
          _EntryTile(
            entry: entry,
            depth: 0,
            expanded: ws.expandedPaths,
            selectedPath: ws.selectedPath,
            onToggle: (path) async {
              if (onToggleFolder != null) {
                await onToggleFolder!(path);
              } else {
                onStateUpdate((s) => s.copyWith(
                      workspace: s.workspace.toggleExpanded(path),
                    ));
              }
            },
            onSelect: (entry) async {
              onStateUpdate((s) => s.copyWith(
                    workspace: s.workspace.copyWith(selectedPath: entry.path),
                  ));
              if (!entry.isFolder) {
                await onOpenFile(entry.path);
              }
            },
          ),
      ],
    );
  }
}

class _EntryTile extends StatelessWidget {
  const _EntryTile({
    required this.entry,
    required this.depth,
    required this.expanded,
    required this.selectedPath,
    required this.onToggle,
    required this.onSelect,
  });

  final WorkspaceEntry entry;
  final int depth;
  final Set<String> expanded;
  final String? selectedPath;
  final ValueChanged<String> onToggle;
  final ValueChanged<WorkspaceEntry> onSelect;

  @override
  Widget build(BuildContext context) {
    final isOpen = expanded.contains(entry.path);
    final selected = selectedPath == entry.path;
    final style = AetherInteraction.resolve(
      selected ? AetherInteractionState.selected : AetherInteractionState.rest,
      base: AetherSurfaces.sidebar,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        InkWell(
          onTap: () {
            if (entry.isFolder) onToggle(entry.path);
            onSelect(entry);
          },
          child: Container(
            height: 26,
            color: style.background,
            padding: EdgeInsets.only(left: 8.0 + depth * 16.0, right: 8),
            child: Row(
              children: [
                if (entry.isFolder)
                  Icon(
                    isOpen ? AetherIcons.chevronDown : AetherIcons.chevronRight,
                    size: 14,
                    color: style.icon,
                  )
                else
                  const SizedBox(width: 14),
                const SizedBox(width: 4),
                Icon(
                  entry.isFolder
                      ? (isOpen ? AetherIcons.folderOpen : AetherIcons.folder)
                      : AetherIcons.fileCode,
                  size: 14,
                  color: style.icon,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    entry.name,
                    style: AetherTypography.fileName.copyWith(
                      color: style.foreground,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (entry.gitStatus != null)
                  Text(
                    entry.gitStatus!,
                    style: AetherTypography.uiMicro.copyWith(
                      color: _gitColor(entry.gitStatus!),
                    ),
                  ),
              ],
            ),
          ),
        ),
        if (entry.isFolder && isOpen)
          for (final child in entry.children)
            _EntryTile(
              entry: child,
              depth: depth + 1,
              expanded: expanded,
              selectedPath: selectedPath,
              onToggle: onToggle,
              onSelect: onSelect,
            ),
      ],
    );
  }

  Color _gitColor(String status) {
    switch (status) {
      case 'A':
        return AetherGit.added;
      case 'M':
        return AetherGit.modified;
      case 'D':
        return AetherGit.deleted;
      case 'U':
        return AetherGit.untracked;
      default:
        return AetherTextColors.tertiary;
    }
  }
}
