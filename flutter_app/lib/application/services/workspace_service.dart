import 'dart:async';

import '../../editor/editor_document.dart';
import '../../state/workspace_state.dart';

/// Workspace operations boundary.
///
/// UI → WorkspaceService → AetherApi → FRB → workspace_core
abstract class WorkspaceService {
  WorkspaceState get state;
  Stream<WorkspaceState> get changes;

  Future<void> openFolder(String path);
  Future<void> refresh();
  Future<void> createFile(String relativePath);
  Future<void> createFolder(String relativePath);
  Future<void> rename(String path, String newName);
  Future<void> delete(String path);
  Future<void> move(String path, String newParentPath);

  Future<EditorDocument> openFile(String path);
  Future<SaveResult> save(EditorDocument document, {bool force = false});
  Future<List<String>> listFiles({String? query});
  Future<List<SearchHit>> search(String query, {String? include, String? exclude});

  void dispose();
}

class SearchHit {
  const SearchHit({
    required this.path,
    required this.line,
    required this.column,
    required this.preview,
  });
  final String path;
  final int line;
  final int column;
  final String preview;
}

/// Prefix used by Rust bridge when expectedVersion mismatches.
const kConflictPrefix = 'conflict:';

String languageIdFor(String path) {
  if (path.endsWith('.dart')) return 'dart';
  if (path.endsWith('.rs')) return 'rust';
  if (path.endsWith('.yaml') || path.endsWith('.yml')) return 'yaml';
  if (path.endsWith('.json')) return 'json';
  if (path.endsWith('.md')) return 'markdown';
  if (path.endsWith('.toml')) return 'toml';
  return 'plaintext';
}

/// External / workspace file change (watcher or poll).
enum WorkspaceChangeKind { created, modified, removed, renamed }

class WorkspaceChangeEvent {
  const WorkspaceChangeEvent({
    required this.path,
    required this.kind,
    this.oldPath,
  });
  final String path;
  final WorkspaceChangeKind kind;
  final String? oldPath;
}
