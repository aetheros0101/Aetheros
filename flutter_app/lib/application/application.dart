export '../editor/editor_document.dart' show SaveResult, SaveConflict, SaveResultKind;

import 'dart:async';

import '../commands/built_in_commands.dart';
import '../commands/command_executor.dart';
import '../commands/command_registry.dart';
import '../editor/editor_session.dart';
import '../navigation/navigation_item.dart';
import '../navigation/navigation_state.dart';
import '../state/app_state.dart';
import '../ui/design/foundation/icons.dart';
import 'services/agent_service.dart';
import 'services/frb_agent_service.dart';
import 'services/frb_workspace_service.dart';
import 'services/git_service.dart';
import 'services/frb_git_service.dart';
import 'services/frb_terminal_service.dart';
import 'services/service_locator.dart';
import 'services/stub_workspace_service.dart';
import 'services/terminal_service.dart';
import 'services/workspace_service.dart';
import 'persistence.dart';
import 'services/workspace_watch_bus.dart';

/// Application composition root.
///
/// Default workspace backend is [FrbWorkspaceService] (AetherApi → FRB →
/// workspace_core). Pass [StubWorkspaceService] in tests.
class AetherApplication {
  AetherApplication({
    WorkspaceService? workspace,
    GitService? git,
    AgentService? agent,
    TerminalService? terminal,
    bool useFrbWorkspace = true,
  }) {
    final ws = workspace ??
        (useFrbWorkspace ? FrbWorkspaceService() : StubWorkspaceService());

    services = ServiceLocator(
      workspace: ws,
      git: git ?? FrbGitService(),
      agent: agent ?? FrbAgentService(),
      terminal: terminal ?? FrbTerminalService(),
    );

    registry.registerAll(builtInCommands());
    executor = CommandExecutor(registry);
    editorSession = EditorSession();

    state = AppState(
      workspace: services.workspace.state,
      git: services.git.state,
      agent: services.agent.state,
      terminal: services.terminal.state,
      navigation: NavigationState(
        items: _defaultNavItems,
        activeId: BuiltInActivities.explorer,
      ),
    );

    _bindServiceSync();
  }

  late final ServiceLocator services;
  final CommandRegistry registry = CommandRegistry();
  late final CommandExecutor executor;
  late final EditorSession editorSession;
  final WorkbenchPersistence persistence = WorkbenchPersistence();
  final WorkspaceWatchBus watchBus = WorkspaceWatchBus();
  final List<void Function(WorkspaceChangeEvent)> _watchListeners = [];

  AppState state = const AppState();

  final List<void Function(AppState)> _listeners = [];
  final List<StreamSubscription<dynamic>> _subs = [];

  static final _defaultNavItems = <NavigationItem>[
    NavigationItem(
        id: BuiltInActivities.explorer,
        title: 'Explorer',
        icon: AetherIcons.files,
        order: 10),
    NavigationItem(
        id: BuiltInActivities.search,
        title: 'Search',
        icon: AetherIcons.search,
        order: 20),
    NavigationItem(
        id: BuiltInActivities.scm,
        title: 'Source Control',
        icon: AetherIcons.branch,
        order: 30),
    NavigationItem(
        id: BuiltInActivities.runDebug,
        title: 'Run and Debug',
        icon: AetherIcons.play,
        order: 40),
    NavigationItem(
        id: BuiltInActivities.agent,
        title: 'Agent',
        icon: AetherIcons.agent,
        order: 50),
  ];

  void _bindServiceSync() {
    _subs.add(services.workspace.changes.listen((ws) {
      updateState((s) => s.copyWith(workspace: ws));
    }));
    _subs.add(services.git.changes.listen((g) {
      updateState((s) => s.copyWith(git: g));
    }));
    _subs.add(services.agent.changes.listen((a) {
      updateState((s) => s.copyWith(agent: a));
    }));
    _subs.add(services.terminal.changes.listen((t) {
      updateState((s) => s.copyWith(terminal: t));
    }));
  }

  /// Call after Rust runtime is ready to bind workspace root + first listing.
  Future<void> initializeWorkspace() async {
    final ws = services.workspace;
    if (ws is FrbWorkspaceService) {
      await ws.bindRuntimeRoot();
    } else {
      await ws.refresh();
    }
    await watchBus.start();
    _subs.add(watchBus.events.listen((ev) {
      for (final l in List.of(_watchListeners)) {
        l(ev);
      }
      // Refresh explorer on structural changes
      if (ev.kind == WorkspaceChangeKind.created ||
          ev.kind == WorkspaceChangeKind.removed ||
          ev.kind == WorkspaceChangeKind.renamed) {
        unawaited(services.workspace.refresh());
      }
    }));
    try {
      await services.git.refresh();
      if (ws is FrbWorkspaceService && services.git is FrbGitService) {
        ws.gitDecorations = (services.git as FrbGitService).decorationMap();
        await ws.refresh();
      }
    } catch (_) {
      // Git optional when not a repo
    }
  }

  /// Expand/collapse folder and reload tree (lazy children via FRB listDir).
  Future<void> toggleFolder(String path) async {
    final ws = services.workspace;
    final next = ws.state.toggleExpanded(path);
    if (ws is FrbWorkspaceService) {
      // Apply expanded set then refresh tree.
      ws.state; // current
      // Refresh rebuilds children for expanded paths only.
      // First push expanded set into service state via refresh path:
      await _setExpandedAndRefresh(ws, next.expandedPaths);
    } else {
      updateState((s) => s.copyWith(workspace: next));
    }
  }

  Future<void> _setExpandedAndRefresh(
    FrbWorkspaceService ws,
    Set<String> expanded,
  ) async {
    // FrbWorkspaceService reads expandedPaths from its own state during _listTree.
    // We mirror expanded into AppState first, then call a targeted refresh.
    updateState((s) => s.copyWith(
          workspace: s.workspace.copyWith(expandedPaths: expanded),
        ));
    // Sync expanded into service-held state by openFolder-style emit:
    // simplest: temporarily rely on AppState.expandedPaths being same —
    // FrbWorkspaceService uses _state.expandedPaths. Update via public API:
    await ws.applyExpanded(expanded);
    await ws.refresh();
  }

  Future<void> openFile(String path, {bool preview = false}) async {
    final doc = await services.workspace.openFile(path);
    editorSession.open(doc);
    updateState(
        (s) => s.copyWith(editor: s.editor.openFile(path, preview: preview)));
  }

  Future<SaveResult> saveFile(String path, {bool force = false}) async {
    final controller = editorSession.controllerFor(path);
    if (controller == null) {
      return SaveResult.error('No open document: $path');
    }
    final result =
        await services.workspace.save(controller.document, force: force);
    if (result.isOk) {
      controller.markSaved(workspaceVersion: result.newVersion);
      updateState((s) => s); // dirty indicator refresh
    }
    return result;
  }

  Future<void> saveAll() async {
    for (final c in editorSession.dirty) {
      await saveFile(c.path);
    }
  }

  void addWatchListener(void Function(WorkspaceChangeEvent) listener) =>
      _watchListeners.add(listener);

  void removeWatchListener(void Function(WorkspaceChangeEvent) listener) =>
      _watchListeners.remove(listener);

  void addListener(void Function(AppState) listener) => _listeners.add(listener);

  void removeListener(void Function(AppState) listener) =>
      _listeners.remove(listener);

  void updateState(AppState Function(AppState) updater) {
    state = updater(state);
    if (state.navigation.items.isEmpty) {
      state = state.copyWith(
        navigation: state.navigation.copyWith(items: _defaultNavItems),
      );
    }
    for (final l in List.of(_listeners)) {
      l(state);
    }
    persistence.scheduleSave(state);
  }

  /// Restore layout + tab paths, then re-open documents from workspace.
  Future<void> restoreLayout() async {
    final saved = await persistence.load();
    if (saved == null) return;

    updateState((s) => s.copyWith(
          workbench: saved.workbench,
          navigation: s.navigation.copyWith(
            activeId: saved.navigation.activeId ?? s.navigation.activeId,
          ),
          editor: saved.editor,
          panel: saved.panel,
        ));

    // Rehydrate document buffers for restored tabs
    for (final g in saved.editor.groups) {
      for (final t in g.tabs) {
        try {
          final doc = await services.workspace.openFile(t.path);
          editorSession.open(doc);
        } catch (_) {
          // File may have been deleted
        }
      }
    }
  }


  /// Save and return result for UI to present conflict dialog.
  Future<SaveResult> saveFileWithPolicy(
    String path, {
    bool force = false,
  }) =>
      saveFile(path, force: force);

  /// Reload open document from workspace (conflict → Reload).
  Future<void> reloadFile(String path) async {
    final doc = await services.workspace.openFile(path);
    final c = editorSession.controllerFor(path);
    if (c != null) {
      c.reloadFromExternal(doc.content, doc.workspaceVersion);
    } else {
      editorSession.open(doc);
    }
    updateState((s) => s);
  }

  /// Poll disk versions for all open editors (external change detection).
  Future<List<String>> detectExternalChanges() async {
    final ws = services.workspace;
    if (ws is! FrbWorkspaceService) return const [];
    final expected = <String, String>{
      for (final c in editorSession.all)
        c.path: c.document.workspaceVersion,
    };
    if (expected.isEmpty) return const [];
    final events = await ws.checkVersions(expected);
    return [
      for (final e in events)
        if (e.kind == WorkspaceChangeKind.modified ||
            e.kind == WorkspaceChangeKind.removed)
          e.path,
    ];
  }

  void dispose() {
    for (final s in _subs) {
      s.cancel();
    }
    _subs.clear();
    persistence.dispose();
    watchBus.dispose();
    services.dispose();
  }
}

// Re-export for callers that need SaveResult type.
