import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/application/application.dart';
import 'package:aetheros_app/application/services/stub_workspace_service.dart';
import 'package:aetheros_app/application/services/git_service.dart';
import 'package:aetheros_app/application/services/agent_service.dart';
import 'package:aetheros_app/application/services/terminal_service.dart';
import 'package:aetheros_app/editor/editor_document.dart';

/// Integration-style test without FRB / Rust.
///
/// Full chain against real workspace_core requires device + cargo test.
/// This verifies Application orchestration: open → edit → save → dirty clear.
void main() {
  test('Explorer→Editor→Save orchestration with stub workspace', () async {
    final app = AetherApplication(
      workspace: StubWorkspaceService(),
      git: StubGitService(),
      agent: StubAgentService(),
      terminal: StubTerminalService(),
      useFrbWorkspace: false,
    );

    await app.openFile('lib/main.dart');
    final c = app.editorSession.controllerFor('lib/main.dart');
    expect(c, isNotNull);
    expect(c!.isDirty, false);

    c.setContent("void main() {\n  print('changed');\n}\n");
    expect(c.isDirty, true);

    final result = await app.saveFile('lib/main.dart');
    expect(result.isOk, true);
    expect(c.isDirty, false);

    // Conflict path: force version skew
    c.document.workspaceVersion = '999';
    c.setContent('conflict me');
    final conflict = await app.saveFile('lib/main.dart');
    expect(conflict.kind, SaveResultKind.conflict);

    final forced = await app.saveFile('lib/main.dart', force: true);
    expect(forced.isOk, true);

    app.dispose();
  });
}
