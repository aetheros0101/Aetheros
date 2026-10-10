/// AetherOS Command — declarative action unit.
///
/// UI never embeds business logic in onPressed; it dispatches a [Command.id].
typedef CommandHandler = Future<void> Function(CommandContext context);

class Command {
  const Command({
    required this.id,
    required this.title,
    required this.handler,
    this.category = 'General',
    this.description,
    this.when,
  });

  /// Stable id: workbench.toggleSidebar, editor.closeTab, …
  final String id;
  final String title;
  final String category;
  final String? description;
  final CommandHandler handler;

  /// Optional enablement predicate.
  final bool Function(CommandContext context)? when;

  bool isEnabled(CommandContext context) => when?.call(context) ?? true;
}
