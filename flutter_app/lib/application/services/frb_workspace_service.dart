import 'dart:async';

import '../../api/aetheros_api.dart';
import '../../editor/editor_document.dart';
import '../../state/workspace_state.dart';
import '../../src/rust/api/aetheros.dart' as rust;
import 'workspace_service.dart';

/// Production WorkspaceService backed by existing AetherApi → FRB → workspace_core.
///
/// Does NOT call the filesystem from Dart. Path security stays in Rust path_guard.
class FrbWorkspaceService implements WorkspaceService {
  FrbWorkspaceService({bool includeHidden = false})
      : _includeHidden = includeHidden;

  final bool _includeHidden;
  WorkspaceState _state = const WorkspaceState();
  final _controller = StreamController<WorkspaceState>.broadcast();

  /// Cache of path → git decoration letter (filled by GitService later).
  Map<String, String> gitDecorations = {};

  @override
  WorkspaceState get state => _state;

  @override
  Stream<WorkspaceState> get changes => _controller.stream;

  void _emit(WorkspaceState next) {
    _state = next;
    if (!_controller.isClosed) _controller.add(next);
  }

  // ─── Folder / tree ───────────────────────────────────────────────────────

  @override
  Future<void> openFolder(String path) async {
    // Rust workspace root is managed by runtime; [path] is informational.
    // Listing always uses workspace-relative paths ("" = root).
    _emit(WorkspaceState(
      rootPath: path,
      rootName: path.replaceAll(r'\', '/').split('/').last,
      isLoading: true,
    ));
    await refresh();
  }

  /// Bind to the runtime workspace root (call once after Rust init).
  Future<void> bindRuntimeRoot() async {
    try {
      final root = await AetherApi.workspaceRoot();
      _emit(_state.copyWith(
        rootPath: root,
        rootName: root.replaceAll(r'\', '/').split('/').last,
      ));
      await refresh();
    } catch (e) {
      _emit(_state.copyWith(error: e.toString(), isLoading: false));
    }
  }

  /// Update expanded set without listing (used before [refresh]).
  Future<void> applyExpanded(Set<String> expanded) async {
    _emit(_state.copyWith(expandedPaths: expanded));
  }

  @override
  Future<void> refresh() async {
    _emit(_state.copyWith(isLoading: true, clearError: true));
    try {
      final entries = await _listTree('');
      _emit(_state.copyWith(
        entries: entries,
        isLoading: false,
        clearError: true,
      ));
    } catch (e) {
      _emit(_state.copyWith(isLoading: false, error: e.toString()));
    }
  }

  /// Shallow list of one directory, mapped to [WorkspaceEntry].
  Future<List<WorkspaceEntry>> _listDir(String dir) async {
    final items = await AetherApi.listDir(dir, includeHidden: _includeHidden);
    final entries = <WorkspaceEntry>[];
    for (final item in items) {
      entries.add(_toEntry(item));
    }
    entries.sort((a, b) {
      if (a.isFolder != b.isFolder) return a.isFolder ? -1 : 1;
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });
    return entries;
  }

  /// Build tree for expanded folders only (lazy).
  Future<List<WorkspaceEntry>> _listTree(String dir) async {
    final top = await _listDir(dir);
    return Future.wait(top.map((e) async {
      if (!e.isFolder) return e;
      if (!_state.expandedPaths.contains(e.path)) return e;
      final children = await _listTree(e.path);
      return e.copyWith(children: children);
    }));
  }

  WorkspaceEntry _toEntry(rust.WsItem item) {
    return WorkspaceEntry(
      path: item.path,
      name: item.name,
      kind: item.isDir ? WorkspaceEntryKind.folder : WorkspaceEntryKind.file,
      gitStatus: gitDecorations[item.path],
    );
  }

  @override
  Future<void> createFile(String relativePath) async {
    await AetherApi.createEntry(relativePath, isDir: false);
    await refresh();
  }

  @override
  Future<void> createFolder(String relativePath) async {
    await AetherApi.createEntry(relativePath, isDir: true);
    await refresh();
  }

  @override
  Future<void> rename(String path, String newName) async {
    final parts = path.replaceAll(r'\', '/').split('/');
    parts[parts.length - 1] = newName;
    final to = parts.join('/');
    await AetherApi.renameEntry(path, to);
    await refresh();
  }

  @override
  Future<void> delete(String path) async {
    await AetherApi.deleteEntry(path);
    await refresh();
  }

  @override
  Future<void> move(String path, String newParentPath) async {
    final name = path.replaceAll(r'\', '/').split('/').last;
    final to = newParentPath.isEmpty ? name : '$newParentPath/$name';
    await AetherApi.renameEntry(path, to);
    await refresh();
  }

  // ─── File content ────────────────────────────────────────────────────────

  @override
  Future<EditorDocument> openFile(String path) async {
    final f = await AetherApi.readFile(path);
    return EditorDocument(
      path: f.path,
      content: f.content,
      version: f.version,
      workspaceVersion: f.version,
      languageId: languageIdFor(f.path),
      isReadonly: f.readonly,
    );
  }

  @override
  Future<SaveResult> save(EditorDocument document, {bool force = false}) async {
    try {
      final written = await AetherApi.writeFile(
        document.path,
        document.content,
        expectedVersion: force ? null : document.workspaceVersion,
      );
      document.markSaved(newWorkspaceVersion: written.version);
      return SaveResult.saved(newVersion: written.version);
    } catch (e) {
      final msg = e.toString();
      if (msg.contains(kConflictPrefix) || msg.startsWith(kConflictPrefix)) {
        return SaveResult.conflict(SaveConflict(
          path: document.path,
          editorVersion: document.workspaceVersion,
          workspaceVersion: 'unknown',
          message: msg,
        ));
      }
      return SaveResult.error(msg);
    }
  }

  // ─── Index / search (P3 text search; Rust index in later phase) ───────────

  @override
  Future<List<String>> listFiles({String? query}) async {
    final all = <String>[];
    await _collectFiles('', all);
    if (query == null || query.trim().isEmpty) return all..sort();
    final q = query.toLowerCase();
    return all.where((p) => p.toLowerCase().contains(q)).toList()..sort();
  }

  Future<void> _collectFiles(String dir, List<String> out) async {
    final items = await AetherApi.listDir(dir, includeHidden: _includeHidden);
    for (final i in items) {
      if (i.isDir) {
        await _collectFiles(i.path, out);
      } else {
        out.add(i.path);
      }
    }
  }

  @override
  Future<List<SearchHit>> search(
    String query, {
    String? include,
    String? exclude,
  }) async {
    if (query.isEmpty) return [];
    final hits = await AetherApi.workspaceSearch(query, maxResults: 500);
    return [
      for (final h in hits)
        SearchHit(
          path: h.path,
          line: h.line,
          column: h.column,
          preview: h.preview,
        ),
    ];
  }

  Future<void> startWatcher() async {
    try {
      await AetherApi.workspaceWatchStart();
    } catch (e) {
      // Watch may be unavailable; non-fatal.
      assert(() {
        // ignore: avoid_print
        print('[FrbWorkspaceService] watch start: $e');
        return true;
      }());
    }
  }

  Future<List<WorkspaceChangeEvent>> drainWatchEvents() async {
    try {
      final hits = await AetherApi.workspaceWatchDrain();
      return [
        for (final h in hits)
          WorkspaceChangeEvent(
            path: h.path,
            kind: switch (h.kind) {
              'created' => WorkspaceChangeKind.created,
              'removed' => WorkspaceChangeKind.removed,
              'renamed' => WorkspaceChangeKind.renamed,
              _ => WorkspaceChangeKind.modified,
            },
            oldPath: h.oldPath,
          ),
      ];
    } catch (e) {
      assert(() {
        // ignore: avoid_print
        print('[FrbWorkspaceService] watch drain: $e');
        return true;
      }());
      return const [];
    }
  }

  /// Compare open document versions against disk (interim until FRB watcher stream).
  ///
  /// Full [WorkspaceWatcher] lives in workspace_core; streaming over FRB is a
  /// thin AetherApi addition. Until then, call this on focus / before save /
  /// on a timer for open tabs.
  Future<List<WorkspaceChangeEvent>> checkVersions(
    Map<String, String> pathToExpectedVersion,
  ) async {
    final changes = <WorkspaceChangeEvent>[];
    for (final entry in pathToExpectedVersion.entries) {
      try {
        final f = await AetherApi.readFile(entry.key);
        if (f.version != entry.value) {
          changes.add(WorkspaceChangeEvent(
            path: entry.key,
            kind: WorkspaceChangeKind.modified,
          ));
        }
      } catch (_) {
        changes.add(WorkspaceChangeEvent(
          path: entry.key,
          kind: WorkspaceChangeKind.removed,
        ));
      }
    }
    return changes;
  }

  @override
  void dispose() {
    _controller.close();
  }
}

