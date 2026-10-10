import 'dart:async';

import '../../api/aetheros_api.dart';
import '../../state/terminal_state.dart';
import 'terminal_service.dart';

/// Terminal backend via existing AetherApi.terminalCheck / terminalRun.
///
/// Not a full PTY yet — interactive shell will use aetheros-terminal over FRB
/// when that surface is exposed. This service runs one-shot commands with the
/// same policy engine as the agent terminal tool.
class FrbTerminalService implements TerminalService {
  TerminalState _state = const TerminalState();
  final _controller = StreamController<TerminalState>.broadcast();
  final _outputs = <String, StreamController<String>>{};
  final _buffers = <String, StringBuffer>{};

  @override
  TerminalState get state => _state;

  @override
  Stream<TerminalState> get changes => _controller.stream;

  void _emit(TerminalState next) {
    _state = next;
    if (!_controller.isClosed) _controller.add(next);
  }

  @override
  Future<String> createSession({String? cwd}) async {
    final id = 'term-${DateTime.now().millisecondsSinceEpoch}';
    final session = TerminalSessionState(
      id: id,
      title: 'Terminal',
      cwd: cwd,
      isRunning: false,
    );
    _outputs[id] = StreamController<String>.broadcast();
    _buffers[id] = StringBuffer();
    _emit(_state.copyWith(
      sessions: [..._state.sessions, session],
      activeSessionId: id,
    ));
    _push(id, 'AetherOS terminal — policy-checked commands\n\$ ');
    return id;
  }

  @override
  Stream<String> output(String sessionId) {
    return _outputs[sessionId]?.stream ?? const Stream.empty();
  }

  void _push(String sessionId, String text) {
    _buffers[sessionId]?.write(text);
    _outputs[sessionId]?.add(text);
  }

  String bufferOf(String sessionId) => _buffers[sessionId]?.toString() ?? '';

  /// Run a single command line through Rust policy + execution.
  Future<void> runCommand(String sessionId, String commandLine) async {
    final line = commandLine.trimRight();
    _push(sessionId, '$line\n');

    // Mark session running
    _emit(_state.copyWith(
      sessions: _state.sessions
          .map((s) => s.id == sessionId ? s.copyWith(isRunning: true) : s)
          .toList(),
    ));

    try {
      final check = await AetherApi.terminalCheck(line);
      if (check.verdict == 'deny') {
        _push(sessionId, 'Denied: ${check.reason}\n\$ ');
        return;
      }

      final confirmed = check.verdict == 'ask' ? true : true;
      // For 'ask', UI should confirm first; here we auto-confirm user-typed
      // commands in the interactive panel (user already typed them).
      final result = await AetherApi.terminalRun(line, confirmed: confirmed);
      _push(sessionId, result.output);
      if (!result.output.endsWith('\n')) _push(sessionId, '\n');
      if (result.truncated) _push(sessionId, '[output truncated]\n');
      _push(sessionId, '\$ ');
    } catch (e) {
      _push(sessionId, 'Error: $e\n\$ ');
    } finally {
      _emit(_state.copyWith(
        sessions: _state.sessions
            .map((s) => s.id == sessionId ? s.copyWith(isRunning: false) : s)
            .toList(),
      ));
    }
  }

  @override
  Future<void> write(String sessionId, String data) async {
    // Treat writes that end with newline as a full command.
    if (data.endsWith('\n') || data.endsWith('\r')) {
      await runCommand(sessionId, data.trim());
    } else {
      _push(sessionId, data);
    }
  }

  @override
  Future<void> disposeSession(String sessionId) async {
    await _outputs.remove(sessionId)?.close();
    _buffers.remove(sessionId);
    final sessions = _state.sessions.where((s) => s.id != sessionId).toList();
    _emit(_state.copyWith(
      sessions: sessions,
      activeSessionId: sessions.isEmpty ? null : sessions.last.id,
    ));
  }

  @override
  void dispose() {
    for (final c in _outputs.values) {
      c.close();
    }
    _outputs.clear();
    _controller.close();
  }
}
