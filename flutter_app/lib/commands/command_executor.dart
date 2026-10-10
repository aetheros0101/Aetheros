import 'package:flutter/foundation.dart';

import 'command.dart';
import 'command_context.dart';
import 'command_registry.dart';

/// Executes commands by id. Unknown / disabled / error results are traceable.
class CommandExecutor {
  CommandExecutor(this.registry);

  final CommandRegistry registry;

  final List<String> history = [];

  Future<CommandResult> execute(
    String id,
    CommandContext context,
  ) async {
    final command = registry.lookup(id);
    if (command == null) {
      debugPrint('[CommandExecutor] unknown command: $id');
      return CommandResult.unknown(id);
    }
    if (!command.isEnabled(context)) {
      debugPrint('[CommandExecutor] disabled command: $id');
      return CommandResult.disabled(id);
    }
    try {
      await command.handler(context);
      history.add(id);
      return CommandResult.ok(id);
    } catch (e, st) {
      debugPrint('[CommandExecutor] error in $id: $e\n$st');
      return CommandResult.error(id, e, st);
    }
  }
}

class CommandResult {
  const CommandResult._(this.id, this.status, {this.error, this.stackTrace});

  final String id;
  final CommandResultStatus status;
  final Object? error;
  final StackTrace? stackTrace;

  bool get isOk => status == CommandResultStatus.ok;

  factory CommandResult.ok(String id) =>
      CommandResult._(id, CommandResultStatus.ok);
  factory CommandResult.unknown(String id) =>
      CommandResult._(id, CommandResultStatus.unknown);
  factory CommandResult.disabled(String id) =>
      CommandResult._(id, CommandResultStatus.disabled);
  factory CommandResult.error(String id, Object error, StackTrace st) =>
      CommandResult._(id, CommandResultStatus.error, error: error, stackTrace: st);
}

enum CommandResultStatus { ok, unknown, disabled, error }
