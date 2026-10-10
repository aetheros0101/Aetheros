import 'dart:async';

import 'package:flutter/material.dart';

import '../../../application/services/frb_terminal_service.dart';
import '../../../application/services/terminal_service.dart';
import '../../../state/app_state.dart';
import '../../design/aether_theme.dart';

class TerminalView extends StatefulWidget {
  const TerminalView({
    super.key,
    required this.state,
    required this.terminal,
  });

  final AppState state;
  final TerminalService terminal;

  @override
  State<TerminalView> createState() => _TerminalViewState();
}

class _TerminalViewState extends State<TerminalView> {
  final _input = TextEditingController();
  final _scroll = ScrollController();
  final _log = StringBuffer();
  StreamSubscription<String>? _sub;
  String? _sessionId;

  @override
  void initState() {
    super.initState();
    _ensureSession();
  }

  Future<void> _ensureSession() async {
    var id = widget.state.terminal.activeSessionId;
    if (id == null) {
      id = await widget.terminal.createSession();
    }
    _sessionId = id;
    _sub?.cancel();
    _sub = widget.terminal.output(id).listen((chunk) {
      setState(() {
        _log.write(chunk);
      });
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scroll.hasClients) {
          _scroll.jumpTo(_scroll.position.maxScrollExtent);
        }
      });
    });
    // Seed buffer if FRB service
    final t = widget.terminal;
    if (t is FrbTerminalService) {
      setState(() {
        _log.clear();
        _log.write(t.bufferOf(id!));
      });
    }
  }

  @override
  void dispose() {
    _sub?.cancel();
    _input.dispose();
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final line = _input.text;
    _input.clear();
    final id = _sessionId;
    if (id == null) return;
    await widget.terminal.write(id, '$line\n');
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: SingleChildScrollView(
            controller: _scroll,
            padding: const EdgeInsets.all(AetherSpacing.sm),
            child: SelectableText(
              _log.toString(),
              style: AetherTypography.terminal.copyWith(
                color: AetherTextColors.primary,
              ),
            ),
          ),
        ),
        Container(
          decoration: BoxDecoration(border: Border(top: AetherBorders.subtle)),
          padding: const EdgeInsets.symmetric(horizontal: AetherSpacing.sm),
          child: TextField(
            controller: _input,
            style: AetherTypography.terminal.copyWith(
              color: AetherTextColors.primary,
            ),
            decoration: const InputDecoration(
              border: InputBorder.none,
              hintText: 'Enter command…',
              isDense: true,
            ),
            onSubmitted: (_) => _submit(),
          ),
        ),
      ],
    );
  }
}
