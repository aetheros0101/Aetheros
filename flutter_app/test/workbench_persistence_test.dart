import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/application/persistence.dart';
import 'package:aetheros_app/state/app_state.dart';
import 'package:aetheros_app/state/workbench_state.dart';
import 'package:aetheros_app/state/editor_state.dart';

void main() {
  group('WorkbenchPersistence layout snapshot', () {
    test('encode/decode preserves sidebar widths and tabs', () {
      final state = AppState(
        workbench: const WorkbenchLayoutState(
          primarySidebarWidth: 280,
          secondarySidebarWidth: 320,
          bottomPanelHeight: 200,
          primarySidebarVisible: true,
          bottomPanelVisible: true,
        ),
        editor: EditorState(
          groups: [
            EditorGroupState(
              tabs: const [
                EditorTabState(id: 'a', path: 'lib/main.dart'),
                EditorTabState(id: 'b', path: 'lib/app.dart', isPreview: true),
              ],
              activeTabIndex: 1,
            ),
          ],
          activeGroupIndex: 0,
        ),
      );

      final snap = WorkbenchPersistence.layoutSnapshot(state);
      expect(snap['workbench'], isA<Map>());
      expect((snap['workbench'] as Map)['primarySidebarWidth'], 280);

      final json = AppStatePersistence.encode(state);
      final restored = AppStatePersistence.decode(json);
      expect(restored.workbench.primarySidebarWidth, 280);
      expect(restored.editor.groups.first.tabs.length, 2);
      expect(restored.editor.groups.first.tabs[1].path, 'lib/app.dart');
    });

    test('corrupt json returns empty AppState', () {
      final s = AppStatePersistence.decode('{not json');
      expect(s.workbench.primarySidebarWidth,
          const WorkbenchLayoutState().primarySidebarWidth);
    });
  });
}
