import 'command.dart';

/// Central registry of all commands. Duplicate ids are rejected.
class CommandRegistry {
  final Map<String, Command> _commands = {};

  void register(Command command) {
    if (_commands.containsKey(command.id)) {
      throw StateError('Duplicate command id: ${command.id}');
    }
    _commands[command.id] = command;
  }

  void registerAll(Iterable<Command> commands) {
    for (final c in commands) {
      register(c);
    }
  }

  Command? lookup(String id) => _commands[id];

  bool contains(String id) => _commands.containsKey(id);

  List<Command> get all => List.unmodifiable(_commands.values);

  List<Command> byCategory(String category) =>
      all.where((c) => c.category == category).toList();

  /// Fuzzy title search for Command Palette.
  List<Command> search(String query) {
    final q = query.trim().toLowerCase();
    if (q.isEmpty) return all;
    return all
        .where((c) =>
            c.id.toLowerCase().contains(q) ||
            c.title.toLowerCase().contains(q) ||
            (c.description?.toLowerCase().contains(q) ?? false))
        .toList();
  }

  void clear() => _commands.clear();

  int get length => _commands.length;
}
