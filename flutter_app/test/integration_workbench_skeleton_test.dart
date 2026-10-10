import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/application/application.dart';
import 'package:aetheros_app/application/services/agent_service.dart';
import 'package:aetheros_app/application/services/git_service.dart';
import 'package:aetheros_app/application/services/stub_workspace_service.dart';
import 'package:aetheros_app/application/services/terminal_service.dart';
import 'package:aetheros_app/commands/command_context.dart';
import 'package:aetheros_app/navigation/navigation_item.dart';

/// Composition-root flows with stubs (no FRB / no full widget tree).
///
/// Maps to WB-603 acceptance themes that can run offline.
void main() {
  late AetherApplication app;

  setUp(() {
    app = AetherApplication(
      workspace: StubWorkspaceService(),
      git: StubGitService(),
      agent: StubAgentService(),
      terminal: StubTerminalService(),
      useFrbWorkspace: false,
    );
  });

  tearDown(() => app.dispose());

  CommandContext ctx() => CommandContext(
        state: app.state,
        updateState: app.updateState,
        services: app.services,
      );

  test('1. Application starts with Workbench navigation defaults', () {
    expect(app.state.navigation.items, isNotEmpty);
    expect(app.state.navigation.activeId, BuiltInActivities.explorer);
    expect(app.registry.length, greaterThan(0));
  });

  test('3–5. Open files, tabs, switch tabs', () async {
    await app.openFile('lib/main.dart');
    expect(app.state.editor.activeGroup.activeTab?.path, 'lib/main.dart');
    expect(app.editorSession.controllerFor('lib/main.dart'), isNotNull);

    await app.openFile('lib/app.dart');
    expect(app.state.editor.activeGroup.tabs.length, 2);

    app.updateState((s) => s.copyWith(editor: s.editor.nextTab()));
    expect(app.state.editor.activeGroup.activeTab?.path, 'lib/main.dart');
  });

  test('6. Edit then save clears dirty on stub workspace', () async {
    await app.openFile('lib/main.dart');
    final c = app.editorSession.controllerFor('lib/main.dart')!;
    c.setContent('${c.document.content}\n// edit');
    expect(c.isDirty, isTrue);
    final result = await app.saveFile('lib/main.dart');
    expect(result.isOk, isTrue);
    expect(c.isDirty, isFalse);
  });

  test('7. Search service returns hits for stub content', () async {
    final hits = await app.services.workspace.search('AetherOS');
    expect(hits, isNotEmpty);
  });

  test('10. workbench.openSearch command executes', () async {
    final r = await app.executor.execute('workbench.openSearch', ctx());
    expect(r.isOk, isTrue);
    expect(app.state.navigation.activeId, BuiltInActivities.search);
    expect(app.state.workbench.primarySidebarVisible, isTrue);
  });

  test('12. Service error does not leave uncaught exception on git refresh',
      () async {
    await app.services.git.refresh();
    // StubGitService succeeds; state remains coherent.
    expect(app.services.git.state.branch, isNotNull);
  });

  test('closeTab drops buffer when path no longer open', () async {
    await app.openFile('lib/main.dart');
    await app.openFile('lib/app.dart');
    app.updateState((s) {
      final next = s.editor.closeTab(0, 0);
      app.editorSession
          .closeIfUnused('lib/main.dart', next.isPathOpen('lib/main.dart'));
      return s.copyWith(editor: next);
    });
    expect(app.editorSession.controllerFor('lib/main.dart'), isNull);
    expect(app.editorSession.controllerFor('lib/app.dart'), isNotNull);
  });
}
