/// AetherOS Application State — Terminal sessions.
class TerminalState {
  const TerminalState({
    this.sessions = const [],
    this.activeSessionId,
  });

  final List<TerminalSessionState> sessions;
  final String? activeSessionId;

  TerminalSessionState? get activeSession {
    if (activeSessionId == null) return null;
    for (final s in sessions) {
      if (s.id == activeSessionId) return s;
    }
    return sessions.isEmpty ? null : sessions.first;
  }

  TerminalState copyWith({
    List<TerminalSessionState>? sessions,
    String? activeSessionId,
  }) {
    return TerminalState(
      sessions: sessions ?? this.sessions,
      activeSessionId: activeSessionId ?? this.activeSessionId,
    );
  }

  Map<String, dynamic> toJson() => {
        'sessions': sessions.map((s) => s.toJson()).toList(),
        'activeSessionId': activeSessionId,
      };

  factory TerminalState.fromJson(Map<String, dynamic> json) {
    final raw = json['sessions'] as List<dynamic>?;
    return TerminalState(
      sessions: raw
              ?.map((e) =>
                  TerminalSessionState.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
      activeSessionId: json['activeSessionId'] as String?,
    );
  }
}

class TerminalSessionState {
  const TerminalSessionState({
    required this.id,
    this.title = 'Terminal',
    this.cwd,
    this.isRunning = false,
  });

  final String id;
  final String title;
  final String? cwd;
  final bool isRunning;

  TerminalSessionState copyWith({
    String? id,
    String? title,
    String? cwd,
    bool? isRunning,
  }) {
    return TerminalSessionState(
      id: id ?? this.id,
      title: title ?? this.title,
      cwd: cwd ?? this.cwd,
      isRunning: isRunning ?? this.isRunning,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'cwd': cwd,
        'isRunning': isRunning,
      };

  factory TerminalSessionState.fromJson(Map<String, dynamic> json) {
    return TerminalSessionState(
      id: json['id'] as String? ?? '',
      title: json['title'] as String? ?? 'Terminal',
      cwd: json['cwd'] as String?,
      isRunning: json['isRunning'] as bool? ?? false,
    );
  }
}
