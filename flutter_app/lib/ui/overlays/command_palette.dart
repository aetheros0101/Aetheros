import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../commands/command.dart';
import '../../commands/command_context.dart';
import '../../commands/command_executor.dart';
import '../../commands/command_registry.dart';
import '../design/aether_theme.dart';

/// Modal command palette (Ctrl/Cmd+Shift+P).
class CommandPalette extends StatefulWidget {
  const CommandPalette({
    super.key,
    required this.registry,
    required this.executor,
    required this.context,
    required this.onClose,
    this.onExecute,
  });

  final CommandRegistry registry;
  final CommandExecutor executor;
  final CommandContext context;
  final VoidCallback onClose;

  /// When set, shell owns execution (overlays, save conflict UI, error SnackBars).
  final Future<void> Function(String commandId)? onExecute;

  @override
  State<CommandPalette> createState() => _CommandPaletteState();
}

class _CommandPaletteState extends State<CommandPalette> {
  final _controller = TextEditingController();
  final _focus = FocusNode();
  List<Command> _results = [];
  int _selected = 0;

  @override
  void initState() {
    super.initState();
    _results = widget.registry.all;
    WidgetsBinding.instance.addPostFrameCallback((_) => _focus.requestFocus());
  }

  @override
  void dispose() {
    _controller.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _query(String q) {
    setState(() {
      _results = widget.registry.search(q);
      _selected = 0;
    });
  }

  Future<void> _run(Command cmd) async {
    widget.onClose();
    if (widget.onExecute != null) {
      await widget.onExecute!(cmd.id);
    } else {
      await widget.executor.execute(cmd.id, widget.context);
    }
  }

  void _move(int delta) {
    if (_results.isEmpty) return;
    setState(() {
      _selected = (_selected + delta).clamp(0, _results.length - 1);
    });
  }

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) return KeyEventResult.ignored;
    final key = event.logicalKey;
    if (key == LogicalKeyboardKey.escape) {
      widget.onClose();
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.arrowDown) {
      _move(1);
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.arrowUp) {
      _move(-1);
      return KeyEventResult.handled;
    }
    if (key == LogicalKeyboardKey.enter) {
      if (_results.isNotEmpty) {
        _run(_results[_selected]);
      }
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    return Material(
      color: AetherSurfaces.overlay,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onClose,
        child: Center(
          child: GestureDetector(
            onTap: () {}, // absorb so backdrop close does not fire
            child: Focus(
              onKeyEvent: _onKey,
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 560, maxHeight: 420),
                child: Container(
                  decoration: BoxDecoration(
                    color: AetherSurfaces.popup,
                    borderRadius: AetherRadius.dialogR,
                    boxShadow: AetherShadows.commandPalette,
                    border: Border.fromBorderSide(AetherBorders.subtle),
                  ),
                  child: Column(
                    children: [
                      Padding(
                        padding: const EdgeInsets.all(AetherSpacing.sm),
                        child: TextField(
                          controller: _controller,
                          focusNode: _focus,
                          style: AetherTypography.uiBody.copyWith(
                            color: AetherTextColors.primary,
                          ),
                          decoration: InputDecoration(
                            hintText: '> Type a command',
                            prefixIcon: Icon(AetherIcons.search,
                                size: 18, color: AetherTextColors.tertiary),
                            border: InputBorder.none,
                            isDense: true,
                          ),
                          onChanged: _query,
                          onSubmitted: (_) {
                            if (_results.isNotEmpty) _run(_results[_selected]);
                          },
                        ),
                      ),
                      Divider(height: 1, color: AetherBorders.subtle.color),
                      Expanded(
                        child: _results.isEmpty
                            ? Center(
                                child: Text(
                                  'No matching commands',
                                  style: AetherTypography.uiCaption.copyWith(
                                    color: AetherTextColors.tertiary,
                                  ),
                                ),
                              )
                            : ListView.builder(
                                itemCount: _results.length,
                                itemBuilder: (context, i) {
                                  final cmd = _results[i];
                                  final sel = i == _selected;
                                  return InkWell(
                                    onTap: () => _run(cmd),
                                    onHover: (_) =>
                                        setState(() => _selected = i),
                                    child: Container(
                                      color: sel
                                          ? AetherInteraction.resolve(
                                              AetherInteractionState.selected,
                                              base: AetherSurfaces.popup,
                                            ).background
                                          : null,
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: AetherSpacing.md,
                                        vertical: AetherSpacing.sm,
                                      ),
                                      child: Row(
                                        children: [
                                          Expanded(
                                            child: Column(
                                              crossAxisAlignment:
                                                  CrossAxisAlignment.start,
                                              children: [
                                                Text(
                                                  cmd.title,
                                                  style: AetherTypography
                                                      .uiBody
                                                      .copyWith(
                                                    color: AetherTextColors
                                                        .primary,
                                                  ),
                                                ),
                                                Text(
                                                  cmd.category,
                                                  style: AetherTypography
                                                      .uiMicro
                                                      .copyWith(
                                                    color: AetherTextColors
                                                        .tertiary,
                                                  ),
                                                ),
                                              ],
                                            ),
                                          ),
                                          Text(
                                            cmd.id,
                                            style: AetherTypography.uiMicro
                                                .copyWith(
                                              color: AetherTextColors.tertiary,
                                            ),
                                          ),
                                        ],
                                      ),
                                    ),
                                  );
                                },
                              ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
