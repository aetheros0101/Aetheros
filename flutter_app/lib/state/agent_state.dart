import '../agent/agent_models.dart';

/// AetherOS Application State — Agent sessions & activity.
class AgentWorkbenchState {
  const AgentWorkbenchState({
    this.sessions = const [],
    this.activeSessionId,
    this.isComposerFocused = false,
  });

  final List<AgentSessionState> sessions;
  final String? activeSessionId;
  final bool isComposerFocused;

  AgentSessionState? get activeSession {
    if (activeSessionId == null) return null;
    for (final s in sessions) {
      if (s.id == activeSessionId) return s;
    }
    return null;
  }

  AgentWorkbenchState copyWith({
    List<AgentSessionState>? sessions,
    String? activeSessionId,
    bool? isComposerFocused,
    bool clearActive = false,
  }) {
    return AgentWorkbenchState(
      sessions: sessions ?? this.sessions,
      activeSessionId:
          clearActive ? null : (activeSessionId ?? this.activeSessionId),
      isComposerFocused: isComposerFocused ?? this.isComposerFocused,
    );
  }

  Map<String, dynamic> toJson() => {
        'sessions': sessions.map((s) => s.toJson()).toList(),
        'activeSessionId': activeSessionId,
      };

  factory AgentWorkbenchState.fromJson(Map<String, dynamic> json) {
    final raw = json['sessions'] as List<dynamic>?;
    return AgentWorkbenchState(
      sessions: raw
              ?.map((e) =>
                  AgentSessionState.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      activeSessionId: json['activeSessionId'] as String?,
    );
  }
}

class AgentSessionState {
  const AgentSessionState({
    required this.id,
    required this.title,
    this.status = 'idle',
    this.messages = const [],
    this.intent,
    this.tasks = const [],
    this.activity = const [],
    this.changes = const [],
    this.verification = const [],
    this.approvals = const [],
    this.pendingApprovalId,
    this.createdAt,
  });

  final String id;
  final String title;
  final String status;
  final List<AgentMessageState> messages;
  final String? intent;
  final List<AgentTaskNode> tasks;
  final List<AgentToolActivity> activity;
  final List<AgentChangeEntry> changes;
  final List<VerificationItem> verification;
  final List<AgentApproval> approvals;
  final String? pendingApprovalId;
  final String? createdAt;

  AgentSessionState copyWith({
    String? id,
    String? title,
    String? status,
    List<AgentMessageState>? messages,
    String? intent,
    List<AgentTaskNode>? tasks,
    List<AgentToolActivity>? activity,
    List<AgentChangeEntry>? changes,
    List<VerificationItem>? verification,
    List<AgentApproval>? approvals,
    String? pendingApprovalId,
    String? createdAt,
    bool clearApproval = false,
  }) {
    return AgentSessionState(
      id: id ?? this.id,
      title: title ?? this.title,
      status: status ?? this.status,
      messages: messages ?? this.messages,
      intent: intent ?? this.intent,
      tasks: tasks ?? this.tasks,
      activity: activity ?? this.activity,
      changes: changes ?? this.changes,
      verification: verification ?? this.verification,
      approvals: approvals ?? this.approvals,
      pendingApprovalId:
          clearApproval ? null : (pendingApprovalId ?? this.pendingApprovalId),
      createdAt: createdAt ?? this.createdAt,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'status': status,
        'messages': messages.map((m) => m.toJson()).toList(),
        'intent': intent,
        'tasks': tasks.map((t) => t.toJson()).toList(),
        'activity': activity.map((a) => a.toJson()).toList(),
        'changes': changes.map((c) => c.toJson()).toList(),
        'verification': verification.map((v) => v.toJson()).toList(),
        'approvals': approvals.map((a) => a.toJson()).toList(),
        'pendingApprovalId': pendingApprovalId,
        'createdAt': createdAt,
      };

  factory AgentSessionState.fromJson(Map<String, dynamic> json) {
    return AgentSessionState(
      id: json['id'] as String? ?? '',
      title: json['title'] as String? ?? '',
      status: json['status'] as String? ?? 'idle',
      messages: (json['messages'] as List<dynamic>?)
              ?.map((e) => AgentMessageState.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      intent: json['intent'] as String?,
      tasks: (json['tasks'] as List<dynamic>?)
              ?.map((e) => AgentTaskNode.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      activity: (json['activity'] as List<dynamic>?)
              ?.map(
                  (e) => AgentToolActivity.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      changes: (json['changes'] as List<dynamic>?)
              ?.map((e) => AgentChangeEntry.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      verification: (json['verification'] as List<dynamic>?)
              ?.map(
                  (e) => VerificationItem.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      approvals: (json['approvals'] as List<dynamic>?)
              ?.map((e) => AgentApproval.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      pendingApprovalId: json['pendingApprovalId'] as String?,
      createdAt: json['createdAt'] as String?,
    );
  }
}

class AgentMessageState {
  const AgentMessageState({
    required this.id,
    required this.role,
    required this.content,
    this.timestamp,
  });

  final String id;
  final String role;
  final String content;
  final String? timestamp;

  Map<String, dynamic> toJson() => {
        'id': id,
        'role': role,
        'content': content,
        'timestamp': timestamp,
      };

  factory AgentMessageState.fromJson(Map<String, dynamic> json) {
    return AgentMessageState(
      id: json['id'] as String? ?? '',
      role: json['role'] as String? ?? 'assistant',
      content: json['content'] as String? ?? '',
      timestamp: json['timestamp'] as String?,
    );
  }
}
