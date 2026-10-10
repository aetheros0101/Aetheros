/// P3 Agent Workbench domain models.

enum AgentTaskStatus { pending, running, completed, failed, skipped }

class AgentTaskNode {
  const AgentTaskNode({
    required this.id,
    required this.title,
    this.status = AgentTaskStatus.pending,
    this.dependsOn = const [],
  });

  final String id;
  final String title;
  final AgentTaskStatus status;
  final List<String> dependsOn;

  AgentTaskNode copyWith({AgentTaskStatus? status}) => AgentTaskNode(
        id: id,
        title: title,
        status: status ?? this.status,
        dependsOn: dependsOn,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'status': status.name,
        'dependsOn': dependsOn,
      };

  factory AgentTaskNode.fromJson(Map<String, dynamic> json) => AgentTaskNode(
        id: json['id'] as String? ?? '',
        title: json['title'] as String? ?? '',
        status: AgentTaskStatus.values.firstWhere(
          (s) => s.name == json['status'],
          orElse: () => AgentTaskStatus.pending,
        ),
        dependsOn: (json['dependsOn'] as List<dynamic>?)
                ?.map((e) => e as String)
                .toList() ??
            const [],
      );
}

enum AgentToolKind { file, search, terminal, git, workspace }

enum AgentToolStatus { pending, running, completed, failed }

class AgentToolActivity {
  const AgentToolActivity({
    required this.id,
    required this.kind,
    required this.summary,
    this.detail,
    this.path,
    this.risk = 'low',
    this.status = AgentToolStatus.pending,
  });

  final String id;
  final AgentToolKind kind;
  final String summary;
  final String? detail;
  final String? path;
  final String risk;
  final AgentToolStatus status;

  Map<String, dynamic> toJson() => {
        'id': id,
        'kind': kind.name,
        'summary': summary,
        'detail': detail,
        'path': path,
        'risk': risk,
        'status': status.name,
      };

  factory AgentToolActivity.fromJson(Map<String, dynamic> json) =>
      AgentToolActivity(
        id: json['id'] as String? ?? '',
        kind: AgentToolKind.values.firstWhere(
          (k) => k.name == json['kind'],
          orElse: () => AgentToolKind.workspace,
        ),
        summary: json['summary'] as String? ?? '',
        detail: json['detail'] as String?,
        path: json['path'] as String?,
        risk: json['risk'] as String? ?? 'low',
        status: AgentToolStatus.values.firstWhere(
          (s) => s.name == json['status'],
          orElse: () => AgentToolStatus.pending,
        ),
      );
}

class AgentChangeEntry {
  const AgentChangeEntry({
    required this.path,
    required this.kind,
    this.diff,
  });

  final String path;
  final String kind;
  final String? diff;

  Map<String, dynamic> toJson() => {
        'path': path,
        'kind': kind,
        'diff': diff,
      };

  factory AgentChangeEntry.fromJson(Map<String, dynamic> json) =>
      AgentChangeEntry(
        path: json['path'] as String? ?? '',
        kind: json['kind'] as String? ?? 'M',
        diff: json['diff'] as String?,
      );
}

enum VerificationStatus { pending, passed, failed, skipped }

class VerificationItem {
  const VerificationItem({
    required this.id,
    required this.title,
    this.status = VerificationStatus.pending,
    this.detail,
  });

  final String id;
  final String title;
  final VerificationStatus status;
  final String? detail;

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'status': status.name,
        'detail': detail,
      };

  factory VerificationItem.fromJson(Map<String, dynamic> json) =>
      VerificationItem(
        id: json['id'] as String? ?? '',
        title: json['title'] as String? ?? '',
        status: VerificationStatus.values.firstWhere(
          (s) => s.name == json['status'],
          orElse: () => VerificationStatus.pending,
        ),
        detail: json['detail'] as String?,
      );
}

enum ApprovalStatus { pending, approved, rejected, expired }

class AgentApproval {
  const AgentApproval({
    required this.id,
    required this.title,
    required this.description,
    this.risk = 'medium',
    this.status = ApprovalStatus.pending,
  });

  final String id;
  final String title;
  final String description;
  final String risk;
  final ApprovalStatus status;

  AgentApproval copyWith({ApprovalStatus? status}) => AgentApproval(
        id: id,
        title: title,
        description: description,
        risk: risk,
        status: status ?? this.status,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'description': description,
        'risk': risk,
        'status': status.name,
      };

  factory AgentApproval.fromJson(Map<String, dynamic> json) => AgentApproval(
        id: json['id'] as String? ?? '',
        title: json['title'] as String? ?? '',
        description: json['description'] as String? ?? '',
        risk: json['risk'] as String? ?? 'medium',
        status: ApprovalStatus.values.firstWhere(
          (s) => s.name == json['status'],
          orElse: () => ApprovalStatus.pending,
        ),
      );
}
