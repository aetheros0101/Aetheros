/// File-backed editor document.
///
/// Version is a String matching Rust workspace document version
/// (used for atomic save conflict detection via expectedVersion).
class EditorDocument {
  EditorDocument({
    required this.path,
    required this.content,
    this.version = '1',
    this.workspaceVersion = '1',
    this.languageId,
    this.isReadonly = false,
  }) : _originalContent = content;

  final String path;
  String content;

  /// Local edit generation (informational).
  String version;

  /// Rust workspace document version — pass as expectedVersion on save.
  String workspaceVersion;

  final String? languageId;
  final bool isReadonly;
  String _originalContent;

  bool get isDirty => content != _originalContent;

  String get fileName {
    final parts = path.replaceAll(r'\', '/').split('/');
    return parts.isEmpty ? path : parts.last;
  }

  void applyEdit(EditorEdit edit) {
    if (isReadonly) return;
    final start = edit.range.start.offset.clamp(0, content.length);
    final end = edit.range.end.offset.clamp(0, content.length);
    content = content.replaceRange(start, end, edit.text);
    version = '${int.tryParse(version) ?? 0 + 1}';
  }

  void markSaved({String? newWorkspaceVersion}) {
    _originalContent = content;
    if (newWorkspaceVersion != null) {
      workspaceVersion = newWorkspaceVersion;
    }
  }

  void reload(String newContent, String newWorkspaceVersion) {
    content = newContent;
    _originalContent = newContent;
    workspaceVersion = newWorkspaceVersion;
  }

  Map<String, dynamic> toJson() => {
        'path': path,
        'version': version,
        'workspaceVersion': workspaceVersion,
        'languageId': languageId,
        'isDirty': isDirty,
      };
}

class EditorPosition {
  const EditorPosition({required this.line, required this.column, this.offset = 0});
  final int line;
  final int column;
  final int offset;
}

class EditorRange {
  const EditorRange({required this.start, required this.end});
  final EditorPosition start;
  final EditorPosition end;
}

class EditorSelection {
  const EditorSelection({required this.anchor, required this.active});
  final EditorPosition anchor;
  final EditorPosition active;
  bool get isEmpty =>
      anchor.line == active.line && anchor.column == active.column;
}

class EditorEdit {
  const EditorEdit({required this.range, required this.text});
  final EditorRange range;
  final String text;
}

class SaveConflict {
  const SaveConflict({
    required this.path,
    required this.editorVersion,
    required this.workspaceVersion,
    this.message,
  });
  final String path;
  final String editorVersion;
  final String workspaceVersion;
  final String? message;
}

enum SaveResultKind { saved, conflict, error }

class SaveResult {
  const SaveResult._(this.kind, {this.conflict, this.error, this.newVersion});
  final SaveResultKind kind;
  final SaveConflict? conflict;
  final String? error;
  final String? newVersion;

  bool get isOk => kind == SaveResultKind.saved;

  factory SaveResult.saved({String? newVersion}) =>
      SaveResult._(SaveResultKind.saved, newVersion: newVersion);
  factory SaveResult.conflict(SaveConflict c) =>
      SaveResult._(SaveResultKind.conflict, conflict: c);
  factory SaveResult.error(String msg) =>
      SaveResult._(SaveResultKind.error, error: msg);
}
