import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/editor/editor_controller.dart';
import 'package:aetheros_app/editor/editor_document.dart';

void main() {
  group('EditorController', () {
    test('setContent marks dirty and supports undo', () {
      final doc = EditorDocument(path: 'a.dart', content: 'hello');
      final c = EditorController(doc);
      expect(c.isDirty, false);

      c.setContent('hello world');
      expect(c.isDirty, true);
      expect(doc.content, 'hello world');

      expect(c.undo(), true);
      expect(doc.content, 'hello');
      expect(c.redo(), true);
      expect(doc.content, 'hello world');
    });

    test('markSaved clears dirty', () {
      final doc = EditorDocument(path: 'a.dart', content: 'x');
      final c = EditorController(doc);
      c.setContent('y');
      c.markSaved(workspaceVersion: '2');
      expect(c.isDirty, false);
      expect(doc.workspaceVersion, '2');
    });

    test('reloadFromExternal resets dirty and history', () {
      final doc = EditorDocument(
          path: 'a.dart', content: 'old', workspaceVersion: '1');
      final c = EditorController(doc);
      c.setContent('local');
      c.reloadFromExternal('disk', '3');
      expect(c.isDirty, false);
      expect(doc.content, 'disk');
      expect(doc.workspaceVersion, '3');
      expect(c.undo(), false);
    });
  });
}
