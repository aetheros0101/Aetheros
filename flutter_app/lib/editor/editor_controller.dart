import 'editor_document.dart';

/// Controls a single open document: edits, selection, undo/redo, dirty state.
///
/// UI must never write [EditorDocument.content] directly — all mutations go
/// through this controller so undo/dirty/version stay consistent.
class EditorController {
  EditorController(this.document)
      : selection = const EditorSelection(
          anchor: EditorPosition(line: 0, column: 0),
          active: EditorPosition(line: 0, column: 0),
        );

  final EditorDocument document;
  EditorSelection selection;

  final List<_HistoryEntry> _undoStack = [];
  final List<_HistoryEntry> _redoStack = [];

  bool get isDirty => document.isDirty;
  String get path => document.path;

  void setSelection(EditorSelection sel) {
    selection = sel;
  }

  /// Full-buffer replace (TextField / external load). Records one undo step.
  void setContent(String newContent, {bool recordHistory = true}) {
    if (document.isReadonly) return;
    if (newContent == document.content) return;
    if (recordHistory) {
      _undoStack.add(_HistoryEntry(document.content));
      _redoStack.clear();
    }
    document.content = newContent;
    document.version = '${(int.tryParse(document.version) ?? 0) + 1}';
  }

  void applyEdit(EditorEdit edit) {
    if (document.isReadonly) return;
    final start = edit.range.start.offset.clamp(0, document.content.length);
    final end = edit.range.end.offset.clamp(0, document.content.length);
    final previous = document.content.substring(start, end);
    _undoStack.add(_HistoryEntry(document.content));
    _redoStack.clear();
    document.content = document.content.replaceRange(start, end, edit.text);
    document.version = '${(int.tryParse(document.version) ?? 0) + 1}';
    // keep selection naive
    selection = EditorSelection(
      anchor: edit.range.start,
      active: EditorPosition(
        line: edit.range.start.line,
        column: edit.range.start.column + edit.text.length,
        offset: start + edit.text.length,
      ),
    );
    // silence unused
    previous;
  }

  void insertText(String text) {
    final a = selection.anchor;
    final b = selection.active;
    final range = EditorRange(
      start: a.offset <= b.offset ? a : b,
      end: a.offset <= b.offset ? b : a,
    );
    applyEdit(EditorEdit(range: range, text: text));
  }

  bool undo() {
    if (_undoStack.isEmpty) return false;
    _redoStack.add(_HistoryEntry(document.content));
    final prev = _undoStack.removeLast();
    document.content = prev.content;
    document.version = '${(int.tryParse(document.version) ?? 0) + 1}';
    return true;
  }

  bool redo() {
    if (_redoStack.isEmpty) return false;
    _undoStack.add(_HistoryEntry(document.content));
    final next = _redoStack.removeLast();
    document.content = next.content;
    document.version = '${(int.tryParse(document.version) ?? 0) + 1}';
    return true;
  }

  void markSaved({String? workspaceVersion}) {
    document.markSaved(newWorkspaceVersion: workspaceVersion);
  }

  /// Reload from external content (watcher / conflict reload) — clears dirty.
  void reloadFromExternal(String content, String workspaceVersion) {
    document.reload(content, workspaceVersion);
    _undoStack.clear();
    _redoStack.clear();
  }
}

class _HistoryEntry {
  _HistoryEntry(this.content);
  final String content;
}
