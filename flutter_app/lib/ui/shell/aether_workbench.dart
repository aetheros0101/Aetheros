import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../application/application.dart';
import '../../commands/command_context.dart';
import '../../commands/command_executor.dart';
import '../../state/app_state.dart';
import '../../state/workbench_state.dart';
import '../design/aether_theme.dart';
import '../overlays/command_palette.dart';
import '../overlays/quick_open.dart';
import '../overlays/save_conflict_dialog.dart';
import '../overlays/unsaved_changes_dialog.dart';
import '../../editor/editor_document.dart';
import 'activity_bar.dart';
import 'bottom_panel.dart';
import 'editor_area.dart';
import 'mobile_nav.dart';
import 'primary_sidebar.dart';
import 'resize.dart';
import 'secondary_sidebar.dart';
import 'status_bar.dart';
import 'responsive.dart';
import 'accessibility_scope.dart';
import 'title_bar.dart';
import '../../navigation/navigation_item.dart';

enum _OverlayKind { none, commandPalette, quickOpen }

class AetherWorkbench extends StatefulWidget {
  const AetherWorkbench({super.key, required this.app});

  final AetherApplication app;

  @override
  State<AetherWorkbench> createState() => _AetherWorkbenchState();
}

class _AetherWorkbenchState extends State<AetherWorkbench> {
  late AppState _state;
  _OverlayKind _overlay = _OverlayKind.none;

  @override
  void initState() {
    super.initState();
    _state = widget.app.state;
    widget.app.addListener(_onAppState);
    widget.app.addWatchListener(_onWatchEvent);
  }

  @override
  void dispose() {
    widget.app.removeWatchListener(_onWatchEvent);
    widget.app.removeListener(_onAppState);
    super.dispose();
  }

  void _onWatchEvent(event) {
    final path = event.path as String?;
    if (path == null) return;
    final c = widget.app.editorSession.controllerFor(path);
    if (c == null) return;
    // Open buffer may be stale — offer reload via existing conflict dialog path
    unawaited(_handleExternalPath(path));
  }

  Future<void> _handleExternalPath(String path) async {
    final c = widget.app.editorSession.controllerFor(path);
    if (c == null || !mounted) return;
    final resolution = await SaveConflictDialog.show(
      context,
      conflict: SaveConflict(
        path: path,
        editorVersion: c.document.workspaceVersion,
        workspaceVersion: 'disk',
        message: 'File changed on disk.',
      ),
      isExternal: true,
    );
    if (!mounted) return;
    if (resolution == ConflictResolution.reload) {
      await widget.app.reloadFile(path);
      setState(() {});
    }
  }

  void _onAppState(AppState s) {
    if (mounted) setState(() => _state = s);
  }

  void _update(AppState Function(AppState) updater) {
    widget.app.updateState(updater);
  }

  CommandContext get _ctx => CommandContext(
        state: _state,
        updateState: _update,
        services: widget.app.services,
      );

  /// Single entry for menu, shortcut, and palette — same behaviour everywhere.
  Future<void> _run(String commandId, {Map<String, Object?> args = const {}}) async {
    // Overlay commands are owned by the shell (need setState / focus).
    if (commandId == 'workbench.commandPalette') {
      setState(() => _overlay = _OverlayKind.commandPalette);
      return;
    }
    if (commandId == 'workbench.quickOpen') {
      setState(() => _overlay = _OverlayKind.quickOpen);
      return;
    }
    // Editor commands that need session / conflict UI stay in the shell.
    if (commandId == 'editor.save') {
      final tab = _state.editor.activeGroup.activeTab;
      if (tab != null) await _saveFile(tab.path);
      return;
    }
    if (commandId == 'editor.saveAll') {
      for (final c in widget.app.editorSession.dirty) {
        await _saveFile(c.path);
      }
      return;
    }
    if (commandId == 'editor.undo') {
      final tab = _state.editor.activeGroup.activeTab;
      final c = tab == null
          ? null
          : widget.app.editorSession.controllerFor(tab.path);
      if (c != null && c.undo()) setState(() {});
      return;
    }
    if (commandId == 'editor.redo') {
      final tab = _state.editor.activeGroup.activeTab;
      final c = tab == null
          ? null
          : widget.app.editorSession.controllerFor(tab.path);
      if (c != null && c.redo()) setState(() {});
      return;
    }
    if (commandId == 'editor.closeTab') {
      final group = _state.editor.activeGroup;
      final tab = group.activeTab;
      if (tab == null) return;
      final dirty =
          widget.app.editorSession.isDirty(tab.path) || tab.isDirty;
      if (dirty) {
        final action = await UnsavedChangesDialog.show(
          context,
          fileName: tab.fileName,
        );
        if (action == null || action == UnsavedChangesAction.cancel) return;
        if (action == UnsavedChangesAction.save) {
          await _saveFile(tab.path);
          if (widget.app.editorSession.isDirty(tab.path)) return;
        }
      }
      final path = tab.path;
      _update((s) {
        final next = s.editor.closeActiveTab();
        widget.app.editorSession
            .closeIfUnused(path, next.isPathOpen(path));
        return s.copyWith(editor: next);
      });
      return;
    }

    final result = await widget.app.executor.execute(
      commandId,
      CommandContext(
        state: _state,
        updateState: _update,
        services: widget.app.services,
        args: args,
      ),
    );

    if (!mounted) return;
    switch (result.status) {
      case CommandResultStatus.unknown:
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Unknown command: $commandId')),
        );
      case CommandResultStatus.error:
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              'Command failed: $commandId — ${result.error}',
            ),
          ),
        );
      case CommandResultStatus.disabled:
      case CommandResultStatus.ok:
        break;
    }
  }

  Future<void> _openFile(String path) async {
    try {
      await widget.app.openFile(path);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not open file: $e')),
      );
    }
  }

  Future<void> _saveFile(String path) async {
    final result = await widget.app.saveFile(path);
    if (!mounted) return;
    if (result.kind == SaveResultKind.conflict && result.conflict != null) {
      final resolution = await SaveConflictDialog.show(
        context,
        conflict: result.conflict!,
      );
      if (!mounted) return;
      switch (resolution) {
        case ConflictResolution.overwrite:
          await widget.app.saveFile(path, force: true);
        case ConflictResolution.reload:
          await widget.app.reloadFile(path);
          setState(() {});
        case ConflictResolution.keepLocal:
        case ConflictResolution.cancel:
        case null:
          break;
      }
    } else if (result.kind == SaveResultKind.error) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(result.error ?? 'Save failed')),
      );
    } else {
      setState(() {});
    }
  }


  void _onResize(ResizeRegion region, double delta) {
    _update((s) {
      final wb = s.workbench;
      switch (region) {
        case ResizeRegion.primarySidebar:
          return s.copyWith(
            workbench: wb.copyWith(
              primarySidebarWidth: (wb.primarySidebarWidth + delta).clamp(
                WorkbenchLayoutState.sidebarMin,
                WorkbenchLayoutState.sidebarMax,
              ),
            ),
          );
        case ResizeRegion.secondarySidebar:
          return s.copyWith(
            workbench: wb.copyWith(
              secondarySidebarWidth: (wb.secondarySidebarWidth - delta).clamp(
                WorkbenchLayoutState.sidebarMin,
                WorkbenchLayoutState.sidebarMax,
              ),
            ),
          );
        case ResizeRegion.bottomPanel:
          return s.copyWith(
            workbench: wb.copyWith(
              bottomPanelHeight: (wb.bottomPanelHeight - delta).clamp(
                WorkbenchLayoutState.panelMin,
                WorkbenchLayoutState.panelMax,
              ),
            ),
          );
      }
    });
  }

  void _mobileSelectActivity(String id) {
    _update((s) {
      final already =
          s.navigation.activeId == id && s.workbench.primarySidebarVisible;
      // Settings is not in navigation.items — set activeId directly.
      final nav = id == BuiltInActivities.settings
          ? s.navigation.copyWith(activeId: id)
          : s.navigation.activate(id);
      return s.copyWith(
        navigation: nav,
        workbench: s.workbench.copyWith(
          primarySidebarVisible: !already,
          secondarySidebarVisible: false,
        ),
      );
    });
  }

  void _mobileOpenAgent() {
    _update((s) => s.copyWith(
          navigation: s.navigation.activate(BuiltInActivities.agent),
          workbench: s.workbench.copyWith(
            secondarySidebarVisible: !s.workbench.secondarySidebarVisible,
            primarySidebarVisible: false,
          ),
        ));
  }

  void _closeMobileDrawer() {
    _update((s) => s.copyWith(
          workbench: s.workbench.copyWith(
            primarySidebarVisible: false,
            secondarySidebarVisible: false,
          ),
        ));
  }

  @override
  Widget build(BuildContext context) {
    final wb = _state.workbench;
    final width = MediaQuery.sizeOf(context).width;
    final mode = layoutModeForWidth(width);
    final a11y = AccessibilityScope.of(context);
    final isMobile = mode == AetherLayoutMode.mobile;

    final showActivity = wb.activityBarVisible && mode.showActivityBar;
    final showPrimary =
        wb.primarySidebarVisible && mode.showPrimarySidebar;
    final showSecondary =
        wb.secondarySidebarVisible && mode.showSecondarySidebar;
    final showPanel = wb.bottomPanelVisible && !isMobile;
    final showStatus = wb.statusBarVisible && !isMobile;
    // Mobile: drawer overlays instead of permanent sidebars.
    final showMobilePrimaryDrawer =
        isMobile && wb.primarySidebarVisible;
    final showMobileAgentDrawer =
        isMobile && wb.secondarySidebarVisible;
    a11y;

    return CallbackShortcuts(
      bindings: _shortcutBindings(),
      child: Focus(
        autofocus: true,
        child: Stack(
          children: [
            Column(
              children: [
                AetherTitleBar(
                  state: _state,
                  onMenu: isMobile
                      ? () => _mobileSelectActivity(
                            _state.navigation.activeId,
                          )
                      : null,
                  onCommandPalette:
                      isMobile ? () => unawaited(_run('workbench.commandPalette')) : null,
                  onQuickOpen:
                      isMobile ? () => unawaited(_run('workbench.quickOpen')) : null,
                ),
                Expanded(
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (showActivity)
                        AetherActivityBar(
                          state: _state,
                          onStateUpdate: _update,
                        ),
                      if (showPrimary) ...[
                        AetherPrimarySidebar(
                          state: _state,
                          onStateUpdate: _update,
                          onOpenFile: _openFile,
                          workspace: widget.app.services.workspace,
                          git: widget.app.services.git,
                          onToggleFolder: widget.app.toggleFolder,
                        ),
                        ResizeHandle(
                          axis: Axis.horizontal,
                          region: ResizeRegion.primarySidebar,
                          onResize: _onResize,
                        ),
                      ],
                      Expanded(
                        child: Column(
                          children: [
                            Expanded(
                              child: AetherEditorArea(
                                state: _state,
                                onStateUpdate: _update,
                                editorSession: widget.app.editorSession,
                                onSave: _saveFile,
                              ),
                            ),
                            if (showPanel) ...[
                              ResizeHandle(
                                axis: Axis.vertical,
                                region: ResizeRegion.bottomPanel,
                                onResize: _onResize,
                              ),
                              AetherBottomPanel(
                                state: _state,
                                onStateUpdate: _update,
                                terminal: widget.app.services.terminal,
                              ),
                            ],
                          ],
                        ),
                      ),
                      if (showSecondary) ...[
                        ResizeHandle(
                          axis: Axis.horizontal,
                          region: ResizeRegion.secondarySidebar,
                          onResize: _onResize,
                        ),
                        AetherSecondarySidebar(
                          state: _state,
                          onStateUpdate: _update,
                          agentService: widget.app.services.agent,
                        ),
                      ],
                    ],
                  ),
                ),
                if (showStatus) AetherStatusBar(state: _state),
                if (isMobile)
                  AetherMobileBottomNav(
                    state: _state,
                    onSelect: _mobileSelectActivity,
                    onOpenAgent: _mobileOpenAgent,
                  ),
              ],
            ),
            // Mobile primary drawer (Explorer / Search / SCM / …)
            if (showMobilePrimaryDrawer)
              Positioned.fill(
                child: Row(
                  children: [
                    Expanded(
                      child: Material(
                        elevation: 8,
                        color: AetherSurfaces.sidebar,
                        child: SafeArea(
                          child: Column(
                            children: [
                              Align(
                                alignment: Alignment.centerRight,
                                child: IconButton(
                                  icon: const Icon(Icons.close, size: 20),
                                  color: AetherTextColors.secondary,
                                  onPressed: _closeMobileDrawer,
                                ),
                              ),
                              Expanded(
                                child: AetherPrimarySidebar(
                                  state: _state,
                                  onStateUpdate: _update,
                                  onOpenFile: (path) async {
                                    await _openFile(path);
                                    _closeMobileDrawer();
                                  },
                                  workspace: widget.app.services.workspace,
                                  git: widget.app.services.git,
                                  onToggleFolder: widget.app.toggleFolder,
                                  expand: true,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                    GestureDetector(
                      onTap: _closeMobileDrawer,
                      child: Container(
                        width: width * 0.2,
                        color: AetherSurfaces.overlay,
                      ),
                    ),
                  ],
                ),
              ),
            // Mobile agent drawer
            if (showMobileAgentDrawer)
              Positioned.fill(
                child: Row(
                  children: [
                    GestureDetector(
                      onTap: _closeMobileDrawer,
                      child: Container(
                        width: width * 0.2,
                        color: AetherSurfaces.overlay,
                      ),
                    ),
                    Expanded(
                      child: Material(
                        elevation: 8,
                        color: AetherSurfaces.sidebar,
                        child: SafeArea(
                          child: Column(
                            children: [
                              Align(
                                alignment: Alignment.centerLeft,
                                child: IconButton(
                                  icon: const Icon(Icons.close, size: 20),
                                  color: AetherTextColors.secondary,
                                  onPressed: _closeMobileDrawer,
                                ),
                              ),
                              Expanded(
                                child: AetherSecondarySidebar(
                                  state: _state,
                                  onStateUpdate: _update,
                                  agentService: widget.app.services.agent,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            if (_overlay == _OverlayKind.commandPalette)
              CommandPalette(
                registry: widget.app.registry,
                executor: widget.app.executor,
                context: _ctx,
                onClose: () => setState(() => _overlay = _OverlayKind.none),
                onExecute: _run,
              ),
            if (_overlay == _OverlayKind.quickOpen)
              QuickOpen(
                workspace: widget.app.services.workspace,
                onOpen: _openFile,
                onClose: () => setState(() => _overlay = _OverlayKind.none),
              ),
          ],
        ),
      ),
    );
  }

  Map<ShortcutActivator, VoidCallback> _shortcutBindings() {
    // Keep in sync with DefaultKeybindings + shell-owned save.
    void go(String id) => unawaited(_run(id));
    return {
      // Sidebar / panel
      const SingleActivator(LogicalKeyboardKey.keyB, control: true):
          () => go('workbench.toggleSidebar'),
      const SingleActivator(LogicalKeyboardKey.keyB, meta: true):
          () => go('workbench.toggleSidebar'),
      const SingleActivator(LogicalKeyboardKey.keyJ, control: true):
          () => go('workbench.togglePanel'),
      const SingleActivator(LogicalKeyboardKey.keyJ, meta: true):
          () => go('workbench.togglePanel'),
      // Tabs
      const SingleActivator(LogicalKeyboardKey.keyW, control: true):
          () => go('editor.closeTab'),
      const SingleActivator(LogicalKeyboardKey.keyW, meta: true):
          () => go('editor.closeTab'),
      const SingleActivator(LogicalKeyboardKey.tab, control: true):
          () => go('editor.nextTab'),
      const SingleActivator(LogicalKeyboardKey.tab, meta: true):
          () => go('editor.nextTab'),
      // Overlays (also via command ids so palette/menu match shortcuts)
      const SingleActivator(LogicalKeyboardKey.keyP, control: true, shift: true):
          () => go('workbench.commandPalette'),
      const SingleActivator(LogicalKeyboardKey.keyP, meta: true, shift: true):
          () => go('workbench.commandPalette'),
      const SingleActivator(LogicalKeyboardKey.keyP, control: true):
          () => go('workbench.quickOpen'),
      const SingleActivator(LogicalKeyboardKey.keyP, meta: true):
          () => go('workbench.quickOpen'),
      // Search view
      const SingleActivator(LogicalKeyboardKey.keyF, control: true, shift: true):
          () => go('workbench.openSearch'),
      const SingleActivator(LogicalKeyboardKey.keyF, meta: true, shift: true):
          () => go('workbench.openSearch'),
      // Agent
      const SingleActivator(LogicalKeyboardKey.keyA, control: true, shift: true):
          () => go('agent.newSession'),
      const SingleActivator(LogicalKeyboardKey.keyA, meta: true, shift: true):
          () => go('agent.newSession'),
      // Save
      const SingleActivator(LogicalKeyboardKey.keyS, control: true):
          () => go('editor.save'),
      const SingleActivator(LogicalKeyboardKey.keyS, meta: true):
          () => go('editor.save'),
      // Escape closes overlay
      const SingleActivator(LogicalKeyboardKey.escape): () {
        if (_overlay != _OverlayKind.none) {
          setState(() => _overlay = _OverlayKind.none);
        }
      },
    };
  }
}
