import 'package:flutter/material.dart';

import '../../application/services/workspace_service.dart';
import '../../navigation/navigation_item.dart';
import '../../state/app_state.dart';
import '../design/aether_theme.dart';
import '../views/explorer/explorer_view.dart';
import '../views/run_debug/run_debug_view.dart';
import '../views/search/search_view.dart';
import '../views/settings/settings_view.dart';
import '../views/source_control/scm_view.dart';
import '../../application/services/git_service.dart';

class AetherPrimarySidebar extends StatelessWidget {
  const AetherPrimarySidebar({
    super.key,
    required this.state,
    required this.onStateUpdate,
    required this.onOpenFile,
    required this.workspace,
    required this.git,
    this.onToggleFolder,
    this.expand = false,
  });

  final AppState state;
  final void Function(AppState Function(AppState)) onStateUpdate;
  final Future<void> Function(String path) onOpenFile;
  final WorkspaceService workspace;
  final GitService git;
  final Future<void> Function(String path)? onToggleFolder;

  /// Mobile drawer: fill parent instead of fixed sidebar width.
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final width = state.workbench.clampedPrimarySidebarWidth;
    final activity = state.navigation.activeId;

    final child = Container(
      color: AetherSurfaces.sidebar,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _SidebarHeader(title: _titleFor(activity)),
          Expanded(child: _contentFor(activity)),
        ],
      ),
    );

    if (expand) return child;
    return SizedBox(width: width, child: child);
  }

  String _titleFor(String id) {
    switch (id) {
      case BuiltInActivities.search:
        return 'SEARCH';
      case BuiltInActivities.scm:
        return 'SOURCE CONTROL';
      case BuiltInActivities.runDebug:
        return 'RUN AND DEBUG';
      case BuiltInActivities.agent:
        return 'AGENT';
      case BuiltInActivities.extensions:
        return 'EXTENSIONS';
      case BuiltInActivities.settings:
        return 'SETTINGS';
      default:
        return 'EXPLORER';
    }
  }

  Widget _contentFor(String id) {
    switch (id) {
      case BuiltInActivities.explorer:
        return ExplorerView(
          state: state,
          onStateUpdate: onStateUpdate,
          onOpenFile: onOpenFile,
          onToggleFolder: onToggleFolder,
        );
      case BuiltInActivities.search:
        return SearchView(
          state: state,
          workspace: workspace,
          onOpenResult: (path, line) => onOpenFile(path),
        );
      case BuiltInActivities.scm:
        return ScmView(
          state: state,
          git: git,
          onOpenFile: onOpenFile,
        );
      case BuiltInActivities.runDebug:
        return const RunDebugView();
      case BuiltInActivities.settings:
        return SettingsView(
          state: state,
          onStateUpdate: onStateUpdate,
        );
      case BuiltInActivities.extensions:
        return const _UnsupportedView(
          title: 'Extensions',
          message: 'Extensions view is not available in this build.',
        );
      case BuiltInActivities.agent:
        // Agent lives in secondary sidebar; primary shows a hint.
        return const _UnsupportedView(
          title: 'Agent',
          message: 'Agent panel is available in the secondary sidebar.',
        );
      default:
        return _UnsupportedView(
          title: id,
          message: 'No view registered for activity "$id".',
        );
    }
  }
}

/// Explicit placeholder for activities that are registered but not implemented.
/// Does not pretend to be functional.
class _UnsupportedView extends StatelessWidget {
  const _UnsupportedView({
    required this.title,
    required this.message,
  });

  final String title;
  final String message;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(AetherSpacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: AetherTypography.uiBody.copyWith(
              color: AetherTextColors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: AetherSpacing.sm),
          Text(
            message,
            style: AetherTypography.uiCaption.copyWith(
              color: AetherTextColors.tertiary,
            ),
          ),
        ],
      ),
    );
  }
}

class _SidebarHeader extends StatelessWidget {
  const _SidebarHeader({required this.title});
  final String title;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 36,
      padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.md),
      alignment: Alignment.centerLeft,
      child: Text(
        title,
        style: AetherTypography.uiMicro.copyWith(
          color: AetherTextColors.secondary,
          letterSpacing: 0.8,
        ),
      ),
    );
  }
}
