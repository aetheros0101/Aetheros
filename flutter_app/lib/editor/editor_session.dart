import 'editor_controller.dart';
import 'editor_document.dart';

/// Open documents registry keyed by path.
class EditorSession {
  final Map<String, EditorController> _controllers = {};

  EditorController? controllerFor(String path) => _controllers[path];

  Iterable<EditorController> get all => _controllers.values;

  List<EditorController> get dirty =>
      _controllers.values.where((c) => c.isDirty).toList();

  EditorController open(EditorDocument doc) {
    final existing = _controllers[doc.path];
    if (existing != null) return existing;
    final c = EditorController(doc);
    _controllers[doc.path] = c;
    return c;
  }

  void close(String path) {
    _controllers.remove(path);
  }

  /// Remove buffer only when no remaining tab references the path.
  void closeIfUnused(String path, bool stillOpenElsewhere) {
    if (!stillOpenElsewhere) {
      _controllers.remove(path);
    }
  }

  bool isDirty(String path) => _controllers[path]?.isDirty ?? false;
}
