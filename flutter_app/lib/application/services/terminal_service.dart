import 'dart:async';

import '../../state/terminal_state.dart';

abstract class TerminalService {
  TerminalState get state;
  Stream<TerminalState> get changes;
  Future<String> createSession({String? cwd});
  Future<void> write(String sessionId, String data);
  Future<void> disposeSession(String sessionId);
  Stream<String> output(String sessionId);
  void dispose();
}

class StubTerminalService implements TerminalService {
  TerminalState _state = const TerminalState();
  final _controller = StreamController<TerminalState>.broadcast();
  final _outputs = <String, StreamController<String>>{};

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
    final session = TerminalSessionState(id: id, cwd: cwd, isRunning: true);
    _outputs[id] = StreamController<String>.broadcast();
    _emit(_state.copyWith(
      sessions: [..._state.sessions, session],
      activeSessionId: id,
    ));
    return id;
  }

  @override
  Future<void> write(String sessionId, String data) async {
    _outputs[sessionId]?.add(data);
  }

  @override
  Future<void> disposeSession(String sessionId) async {
    await _outputs.remove(sessionId)?.close();
    _emit(_state.copyWith(
      sessions: _state.sessions.where((s) => s.id != sessionId).toList(),
    ));
  }

  @override
  Stream<String> output(String sessionId) =>
      _outputs[sessionId]?.stream ?? const Stream.empty();

  @override
  void dispose() {
    for (final c in _outputs.values) {
      c.close();
    }
    _outputs.clear();
    _controller.close();
  }
}
