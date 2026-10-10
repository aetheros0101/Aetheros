import 'dart:async';

import '../../agent/agent_models.dart';
import '../../state/agent_state.dart';

/// Owns all agent session lifecycle including plan/tasks/activity/approvals.
abstract class AgentService {
  AgentWorkbenchState get state;
  Stream<AgentWorkbenchState> get changes;
  Future<AgentSessionState> newSession({String? title});
  Future<void> sendMessage(String sessionId, String content);
  Future<void> stop(String sessionId);
  Future<void> approve(String approvalId);
  Future<void> reject(String approvalId);
  Future<void> setActiveSession(String? sessionId);
  void dispose();
}

class StubAgentService implements AgentService {
  AgentWorkbenchState _state = const AgentWorkbenchState();
  final _controller = StreamController<AgentWorkbenchState>.broadcast();

  @override
  AgentWorkbenchState get state => _state;

  @override
  Stream<AgentWorkbenchState> get changes => _controller.stream;

  void _emit(AgentWorkbenchState next) {
    _state = next;
    if (!_controller.isClosed) _controller.add(next);
  }

  @override
  Future<AgentSessionState> newSession({String? title}) async {
    final id = 'session-${DateTime.now().millisecondsSinceEpoch}';
    final session = AgentSessionState(
      id: id,
      title: title ?? 'New Session',
      status: 'idle',
      createdAt: DateTime.now().toIso8601String(),
    );
    _emit(_state.copyWith(
      sessions: [..._state.sessions, session],
      activeSessionId: id,
    ));
    return session;
  }

  @override
  Future<void> sendMessage(String sessionId, String content) async {
    final sessions = _state.sessions.map((s) {
      if (s.id != sessionId) return s;
      final msg = AgentMessageState(
        id: 'msg-${DateTime.now().millisecondsSinceEpoch}',
        role: 'user',
        content: content,
        timestamp: DateTime.now().toIso8601String(),
      );
      // Stub: create a simple plan from the prompt.
      final tasks = [
        const AgentTaskNode(id: 't1', title: 'Analyze workspace', status: AgentTaskStatus.completed),
        const AgentTaskNode(id: 't2', title: 'Plan changes', status: AgentTaskStatus.running),
        const AgentTaskNode(id: 't3', title: 'Apply edits', status: AgentTaskStatus.pending),
        const AgentTaskNode(id: 't4', title: 'Verify', status: AgentTaskStatus.pending),
      ];
      final activity = [
        AgentToolActivity(
          id: 'a1',
          kind: AgentToolKind.search,
          summary: 'Search workspace',
          detail: content,
          status: AgentToolStatus.completed,
        ),
      ];
      return s.copyWith(
        messages: [...s.messages, msg],
        status: 'planning',
        intent: content,
        tasks: tasks,
        activity: activity,
        title: content.length > 40 ? '${content.substring(0, 40)}…' : content,
      );
    }).toList();
    _emit(_state.copyWith(sessions: sessions));
  }

  @override
  Future<void> stop(String sessionId) async {
    final sessions = _state.sessions
        .map((s) => s.id == sessionId ? s.copyWith(status: 'cancelled') : s)
        .toList();
    _emit(_state.copyWith(sessions: sessions));
  }

  @override
  Future<void> approve(String approvalId) async {
    final sessions = _state.sessions.map((s) {
      final approvals = s.approvals
          .map((a) => a.id == approvalId
              ? a.copyWith(status: ApprovalStatus.approved)
              : a)
          .toList();
      return s.copyWith(approvals: approvals, clearApproval: true);
    }).toList();
    _emit(_state.copyWith(sessions: sessions));
  }

  @override
  Future<void> reject(String approvalId) async {
    final sessions = _state.sessions.map((s) {
      final approvals = s.approvals
          .map((a) => a.id == approvalId
              ? a.copyWith(status: ApprovalStatus.rejected)
              : a)
          .toList();
      return s.copyWith(approvals: approvals, clearApproval: true);
    }).toList();
    _emit(_state.copyWith(sessions: sessions));
  }

  @override
  Future<void> setActiveSession(String? sessionId) async {
    _emit(_state.copyWith(activeSessionId: sessionId));
  }

  @override
  void dispose() {
    _controller.close();
  }
}
