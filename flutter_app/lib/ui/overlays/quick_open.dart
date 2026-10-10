import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../application/services/workspace_service.dart';
import '../design/aether_theme.dart';

/// Quick Open file picker (Ctrl/Cmd+P).
class QuickOpen extends StatefulWidget {
  const QuickOpen({
    super.key,
    required this.workspace,
    required this.onOpen,
    required this.onClose,
  });

  final WorkspaceService workspace;
  final ValueChanged<String> onOpen;
  final VoidCallback onClose;

  @override
  State<QuickOpen> createState() => _QuickOpenState();
}

class _QuickOpenState extends State<QuickOpen> {
  final _controller = TextEditingController();
  final _focus = FocusNode();
  List<String> _results = [];
  int _selected = 0;
  bool _loading = false;
  String? _error;
  int _gen = 0;

  @override
  void initState() {
    super.initState();
    _load('');
    WidgetsBinding.instance.addPostFrameCallback((_) => _focus.requestFocus());
  }

  @override
  void dispose() {
    _controller.dispose();
    _focus.dispose();
    super.dispose();
  }

  Future<void> _load(String q) async {
    final gen = ++_gen;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final files = await widget.workspace.listFiles(query: q);
      if (!mounted || gen != _gen) return;
      setState(() {
        _results = files;
        _selected = 0;
        _loading = false;
      });
    } catch (e) {
      if (!mounted || gen != _gen) return;
      setState(() {
        _results = [];
        _loading = false;
        _error = e.toString();
      });
    }
  }

  String _fileName(String path) {
    final parts = path.replaceAll('\\', '/').split('/');
    return parts.isEmpty ? path : parts.last;
  }

  void _openSelected() {
    if (_results.isEmpty) return;
    widget.onOpen(_results[_selected]);
    widget.onClose();
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
      _openSelected();
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
            onTap: () {},
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
                            hintText: 'Search files by name',
                            prefixIcon: Icon(AetherIcons.file,
                                size: 18, color: AetherTextColors.tertiary),
                            border: InputBorder.none,
                            isDense: true,
                          ),
                          onChanged: _load,
                          onSubmitted: (_) => _openSelected(),
                        ),
                      ),
                      if (_loading)
                        const LinearProgressIndicator(minHeight: 2)
                      else
                        Divider(height: 1, color: AetherBorders.subtle.color),
                      Expanded(child: _body()),
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

  Widget _body() {
    if (_error != null) {
      return Padding(
        padding: const EdgeInsets.all(AetherSpacing.md),
        child: Text(
          _error!,
          style: AetherTypography.uiCaption.copyWith(color: AetherStatus.danger),
        ),
      );
    }
    if (!_loading && _results.isEmpty) {
      return Center(
        child: Text(
          'No files found',
          style: AetherTypography.uiCaption.copyWith(
            color: AetherTextColors.tertiary,
          ),
        ),
      );
    }
    return ListView.builder(
      itemCount: _results.length,
      itemBuilder: (context, i) {
        final path = _results[i];
        final sel = i == _selected;
        return InkWell(
          onTap: () {
            widget.onOpen(path);
            widget.onClose();
          },
          onHover: (_) => setState(() => _selected = i),
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
                Icon(AetherIcons.fileCode,
                    size: 14, color: AetherTextColors.secondary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    _fileName(path),
                    style: AetherTypography.fileName.copyWith(
                      color: AetherTextColors.primary,
                    ),
                  ),
                ),
                Flexible(
                  child: Text(
                    path,
                    style: AetherTypography.filePath.copyWith(
                      color: AetherTextColors.tertiary,
                    ),
                    overflow: TextOverflow.ellipsis,
                    textAlign: TextAlign.right,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
