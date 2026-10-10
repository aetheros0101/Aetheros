import 'package:flutter_test/flutter_test.dart';

import 'package:aetheros_app/commands/command.dart';
import 'package:aetheros_app/commands/command_context.dart';
import 'package:aetheros_app/commands/command_executor.dart';
import 'package:aetheros_app/commands/command_registry.dart';
import 'package:aetheros_app/commands/built_in_commands.dart';
import 'package:aetheros_app/application/services/service_locator.dart';
import 'package:aetheros_app/application/services/stub_workspace_service.dart';
import 'package:aetheros_app/application/services/git_service.dart';
import 'package:aetheros_app/application/services/agent_service.dart';
import 'package:aetheros_app/application/services/terminal_service.dart';
import 'package:aetheros_app/state/app_state.dart';

void main() {
  late CommandRegistry registry;
  late CommandExecutor executor;
  late ServiceLocator services;

  setUp(() {
    registry = CommandRegistry();
    registry.registerAll(builtInCommands());
    executor = CommandExecutor(registry);
    services = ServiceLocator(
      workspace: StubWorkspaceService(),
      git: StubGitService(),
      agent: StubAgentService(),
      terminal: StubTerminalService(),
    );
  });

  CommandContext ctx({AppState? state}) => CommandContext(
        state: state ?? const AppState(),
        updateState: (_) {},
        services: services,
      );

  test('duplicate command id is rejected', () {
    expect(
      () => registry.register(
        Command(
          id: 'workbench.toggleSidebar',
          title: 'Dup',
          handler: (_) async {},
        ),
      ),
      throwsStateError,
    );
  });

  test('lookup returns registered command', () {
    expect(registry.lookup('workbench.toggleSidebar'), isNotNull);
    expect(registry.lookup('no.such.command'), isNull);
  });

  test('search filters by title and id', () {
    final hits = registry.search('sidebar');
    expect(hits.isNotEmpty, isTrue);
  });

  test('execute unknown returns unknown status', () async {
    final r = await executor.execute('does.not.exist', ctx());
    expect(r.status, CommandResultStatus.unknown);
  });

  test('execute toggleSidebar succeeds', () async {
    var updated = false;
    final c = CommandContext(
      state: const AppState(),
      updateState: (_) => updated = true,
      services: services,
    );
    final r = await executor.execute('workbench.toggleSidebar', c);
    expect(r.isOk, isTrue);
    expect(updated, isTrue);
  });

  test('built-in command ids are unique', () {
    final ids = builtInCommands().map((c) => c.id).toList();
    expect(ids.toSet().length, ids.length);
  });
}
