import 'dart:async';

import '../../editor/editor_document.dart';
import '../../state/workspace_state.dart';
import 'workspace_service.dart';

/// In-memory stub for tests / offline UI development without Rust.
class StubWorkspaceService implements WorkspaceService {
  StubWorkspaceService() {
    _files = {
      'lib/main.dart': "void main() {\n  print('AetherOS');\n}\n",
      'lib/app.dart': "class App {\n  void run() {}\n}\n",
      'pubspec.yaml': 'name: demo\nversion: 0.1.0\n',
      'README.md': '# Demo\n',
    };
    _versions = {for (final k in _files.keys) k: '1'};
    _state = WorkspaceState(
      rootPath: '/demo/project',
      rootName: 'project',
      entries: const [
        WorkspaceEntry(
          path: 'lib',
          name: 'lib',
          kind: WorkspaceEntryKind.folder,
          children: [
            WorkspaceEntry(
                path: 'lib/main.dart',
                name: 'main.dart',
                kind: WorkspaceEntryKind.file),
            WorkspaceEntry(
                path: 'lib/app.dart',
                name: 'app.dart',
                kind: WorkspaceEntryKind.file,
                gitStatus: 'M'),
          ],
        ),
        WorkspaceEntry(
            path: 'pubspec.yaml',
            name: 'pubspec.yaml',
            kind: WorkspaceEntryKind.file),
        WorkspaceEntry(
            path: 'README.md',
            name: 'README.md',
            kind: WorkspaceEntryKind.file,
            gitStatus: 'A'),
      ],
      expandedPaths: {'lib'},
    );
  }

  late WorkspaceState _state;
  late Map<String, String> _files;
  late Map<String, String> _versions;
  final _controller = StreamController<WorkspaceState>.broadcast();

  @override
  WorkspaceState get state => _state;

  @override
  Stream<WorkspaceState> get changes => _controller.stream;

  void _emit(WorkspaceState n) {
    _state = n;
    if (!_controller.isClosed) _controller.add(n);
  }

  @override
  Future<void> openFolder(String path) async {
    _emit(WorkspaceState(rootPath: path, rootName: path.split('/').last));
  }

  @override
  Future<void> refresh() async => _emit(_state);

  @override
  Future<void> createFile(String relativePath) async {
    _files[relativePath] = '';
    _versions[relativePath] = '1';
  }

  @override
  Future<void> createFolder(String relativePath) async {}

  @override
  Future<void> rename(String path, String newName) async {}

  @override
  Future<void> delete(String path) async {
    _files.remove(path);
    _versions.remove(path);
  }

  @override
  Future<void> move(String path, String newParentPath) async {}

  @override
  Future<EditorDocument> openFile(String path) async {
    return EditorDocument(
      path: path,
      content: _files[path] ?? '',
      version: _versions[path] ?? '1',
      workspaceVersion: _versions[path] ?? '1',
      languageId: languageIdFor(path),
    );
  }

  @override
  Future<SaveResult> save(EditorDocument document, {bool force = false}) async {
    final current = _versions[document.path] ?? '1';
    if (!force && document.workspaceVersion != current) {
      return SaveResult.conflict(SaveConflict(
        path: document.path,
        editorVersion: document.workspaceVersion,
        workspaceVersion: current,
      ));
    }
    _files[document.path] = document.content;
    final next = '${(int.tryParse(current) ?? 1) + 1}';
    _versions[document.path] = next;
    document.markSaved(newWorkspaceVersion: next);
    return SaveResult.saved(newVersion: next);
  }

  @override
  Future<List<String>> listFiles({String? query}) async {
    final all = _files.keys.toList()..sort();
    if (query == null || query.isEmpty) return all;
    final q = query.toLowerCase();
    return all.where((p) => p.toLowerCase().contains(q)).toList();
  }

  @override
  Future<List<SearchHit>> search(String query,
      {String? include, String? exclude}) async {
    final hits = <SearchHit>[];
    for (final e in _files.entries) {
      final lines = e.value.split('\n');
      for (var i = 0; i < lines.length; i++) {
        final c = lines[i].indexOf(query);
        if (c >= 0) {
          hits.add(SearchHit(
              path: e.key, line: i + 1, column: c + 1, preview: lines[i].trim()));
        }
      }
    }
    return hits;
  }

  @override
  void dispose() => _controller.close();
}
