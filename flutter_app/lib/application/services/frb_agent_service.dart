import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../agent/agent_models.dart';
import '../../api/aetheros_api.dart';
import '../../core/agent_capabilities.dart';
import '../../state/agent_state.dart';
import 'agent_service.dart';

/// Agent Workbench backend via existing AetherApi → FRB → Rust agent runtime.
///
/// Flow:
///   User message → startAgent(objective)
///   Poll getAgentStatus + listPendingApprovals + listAuditEvents
///   Approve/Reject → respondToApproval
///
/// No second bridge. Security decisions stay in Rust.
class FrbAgentService implements AgentService {
  FrbAgentService({
    this.maxSteps = 10,
    this.maxTokens = 4096,
    this.defaultCapabilities = kDefaultWorkbenchCapabilities,
    this.pollInterval = const Duration(milliseconds: 800),
  });

  final int maxSteps;
  final int maxTokens;
  final List<String> defaultCapabilities;
  final Duration pollInterval;

  AgentWorkbenchState _state = const AgentWorkbenchState();
  final _controller = StreamController<AgentWorkbenchState>.broadcast();
  final _pollers = <String, Timer>{};

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
    final id = 'local-${DateTime.now().millisecondsSinceEpoch}';
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
  Future<void> setActiveSession(String? sessionId) async {
    _emit(_state.copyWith(activeSessionId: sessionId));
  }

  /// Start Rust agent with [content] as objective.
  @override
  Future<void> sendMessage(String sessionId, String content) async {
    final text = content.trim();
    if (text.isEmpty) return;

    final sessions = _state.sessions.map((s) {
      if (s.id != sessionId) return s;
      final msg = AgentMessageState(
        id: 'msg-${DateTime.now().millisecondsSinceEpoch}',
        role: 'user',
        content: text,
        timestamp: DateTime.now().toIso8601String(),
      );
      return s.copyWith(
        messages: [...s.messages, msg],
        intent: text,
        status: 'starting',
        title: text.length > 48 ? '${text.substring(0, 48)}…' : text,
        tasks: [
          const AgentTaskNode(
            id: 'plan',
            title: 'Plan',
            status: AgentTaskStatus.running,
          ),
          const AgentTaskNode(id: 'execute', title: 'Execute tools'),
          const AgentTaskNode(id: 'verify', title: 'Verify'),
        ],
      );
    }).toList();
    _emit(_state.copyWith(sessions: sessions));

    try {
      final started = await AetherApi.startAgent(
        objective: text,
        maxSteps: maxSteps,
        maxTokens: maxTokens,
        capabilities: defaultCapabilities,
      );

      // Re-key session id to execution id so polls align with Rust.
      final updated = _state.sessions.map((s) {
        if (s.id != sessionId) return s;
        return s.copyWith(
          id: started.executionId,
          status: started.status.isEmpty ? 'running' : started.status,
        );
      }).toList();

      final active = _state.activeSessionId == sessionId
          ? started.executionId
          : _state.activeSessionId;

      _emit(_state.copyWith(sessions: updated, activeSessionId: active));
      _startPoll(started.executionId);
    } catch (e) {
      _patchSession(sessionId, (s) => s.copyWith(
            status: 'failed',
            messages: [
              ...s.messages,
              AgentMessageState(
                id: 'err-${DateTime.now().millisecondsSinceEpoch}',
                role: 'system',
                content: 'Failed to start agent: $e',
              ),
            ],
          ));
    }
  }

  void _startPoll(String executionId) {
    _pollers[executionId]?.cancel();
    _pollers[executionId] = Timer.periodic(pollInterval, (_) async {
      await _pollOnce(executionId);
    });
    // Immediate first tick
    unawaited(_pollOnce(executionId));
  }

  Future<void> _pollOnce(String executionId) async {
    try {
      final status = await AetherApi.getAgentStatus(executionId);
      final approvals = await AetherApi.listPendingApprovals();
      final related = approvals
          .where((a) => a.executionId == executionId)
          .map((a) => AgentApproval(
                id: a.id,
                title: a.toolName,
                description: a.reason.isNotEmpty
                    ? a.reason
                    : a.arguments.join(' '),
                risk: 'medium',
                status: ApprovalStatus.pending,
              ))
          .toList();

      List<AgentToolActivity> activity = const [];
      try {
        final audit = await AetherApi.listAuditEvents(executionId);
        activity = [
          for (final e in audit)
            AgentToolActivity(
              id: e.id,
              kind: _toolKind('${e.kindLabel} ${e.summary}'),
              summary: e.summary.isNotEmpty ? e.summary : e.kindLabel,
              status: AgentToolStatus.completed,
            ),
        ];
      } catch (e) {
        // Audit is optional enrichment; session poll continues without it.
        debugPrint('[FrbAgentService] listAuditEvents: $e');
      }

      final taskStatus = _mapTasks(status.status);
      final done = status.status == 'completed' ||
          status.status == 'failed' ||
          status.status == 'cancelled';

      _patchSession(executionId, (s) {
        return s.copyWith(
          status: status.status,
          tasks: taskStatus,
          activity: activity.isEmpty ? s.activity : activity,
          approvals: related.isEmpty ? s.approvals : related,
          pendingApprovalId: status.pendingApprovalId,
          messages: status.error != null &&
                  !s.messages.any((m) => m.content == status.error)
              ? [
                  ...s.messages,
                  AgentMessageState(
                    id: 'err-$executionId',
                    role: 'system',
                    content: status.error!,
                  ),
                ]
              : s.messages,
        );
      });

      if (done) {
        _pollers.remove(executionId)?.cancel();
      }
    } catch (e) {
      // Transient poll errors: keep last known session state, log for diagnosis.
      debugPrint('[FrbAgentService] poll $executionId: $e');
    }
  }

  List<AgentTaskNode> _mapTasks(String status) {
    switch (status) {
      case 'completed':
        return const [
          AgentTaskNode(
              id: 'plan', title: 'Plan', status: AgentTaskStatus.completed),
          AgentTaskNode(
              id: 'execute',
              title: 'Execute tools',
              status: AgentTaskStatus.completed),
          AgentTaskNode(
              id: 'verify', title: 'Verify', status: AgentTaskStatus.completed),
        ];
      case 'failed':
        return const [
          AgentTaskNode(
              id: 'plan', title: 'Plan', status: AgentTaskStatus.completed),
          AgentTaskNode(
              id: 'execute',
              title: 'Execute tools',
              status: AgentTaskStatus.failed),
          AgentTaskNode(
              id: 'verify', title: 'Verify', status: AgentTaskStatus.skipped),
        ];
      case 'pending_approval':
        return const [
          AgentTaskNode(
              id: 'plan', title: 'Plan', status: AgentTaskStatus.completed),
          AgentTaskNode(
              id: 'execute',
              title: 'Execute tools',
              status: AgentTaskStatus.running),
          AgentTaskNode(id: 'verify', title: 'Verify'),
        ];
      default:
        return const [
          AgentTaskNode(
              id: 'plan', title: 'Plan', status: AgentTaskStatus.completed),
          AgentTaskNode(
              id: 'execute',
              title: 'Execute tools',
              status: AgentTaskStatus.running),
          AgentTaskNode(id: 'verify', title: 'Verify'),
        ];
    }
  }

  AgentToolKind _toolKind(String raw) {
    final l = raw.toLowerCase();
    if (l.contains('terminal') || l.contains('command')) {
      return AgentToolKind.terminal;
    }
    if (l.contains('git')) return AgentToolKind.git;
    if (l.contains('search')) return AgentToolKind.search;
    if (l.contains('file') || l.contains('write') || l.contains('read')) {
      return AgentToolKind.file;
    }
    return AgentToolKind.workspace;
  }

  String _auditSummary(dynamic event) {
    try {
      // AuditEvent fields vary; use toString fallback with light parse
      final s = event.toString();
      if (event is Object) {
        // ignore: avoid_dynamic_calls
        final tool = _tryField(event, 'toolName') ??
            _tryField(event, 'action') ??
            _tryField(event, 'kind');
        final detail = _tryField(event, 'detail') ??
            _tryField(event, 'message') ??
            _tryField(event, 'summary');
        if (tool != null) {
          return detail != null ? '$tool — $detail' : tool.toString();
        }
      }
      return s.length > 120 ? '${s.substring(0, 120)}…' : s;
    } catch (_) {
      return 'activity';
    }
  }

  dynamic _tryField(Object event, String name) {
    try {
      // ignore: avoid_dynamic_calls
      final v = (event as dynamic);
      switch (name) {
        case 'toolName':
          return v.toolName;
        case 'action':
          return v.action;
        case 'kind':
          return v.kind;
        case 'detail':
          return v.detail;
        case 'message':
          return v.message;
        case 'summary':
          return v.summary;
      }
    } catch (_) {}
    return null;
  }

  void _patchSession(
    String id,
    AgentSessionState Function(AgentSessionState) fn,
  ) {
    final sessions = _state.sessions.map((s) => s.id == id ? fn(s) : s).toList();
    _emit(_state.copyWith(sessions: sessions));
  }

  @override
  Future<void> stop(String sessionId) async {
    _pollers.remove(sessionId)?.cancel();
    _patchSession(sessionId, (s) => s.copyWith(status: 'cancelled'));
  }

  @override
  Future<void> approve(String approvalId) async {
    await AetherApi.respondToApproval(approvalId: approvalId, approved: true);
    for (final s in _state.sessions) {
      if (s.pendingApprovalId == approvalId ||
          s.approvals.any((a) => a.id == approvalId)) {
        _startPoll(s.id);
      }
    }
    // Refresh approvals on active session
    final active = _state.activeSessionId;
    if (active != null) await _pollOnce(active);
  }

  @override
  Future<void> reject(String approvalId) async {
    await AetherApi.respondToApproval(approvalId: approvalId, approved: false);
    final active = _state.activeSessionId;
    if (active != null) await _pollOnce(active);
  }

  @override
  void dispose() {
    for (final t in _pollers.values) {
      t.cancel();
    }
    _pollers.clear();
    _controller.close();
  }
}
