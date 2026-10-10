/// AetherOS Application State — Diagnostics / problems.
class ProblemsState {
  const ProblemsState({
    this.items = const [],
  });

  final List<ProblemItem> items;

  int get errorCount =>
      items.where((i) => i.severity == ProblemSeverity.error).length;
  int get warningCount =>
      items.where((i) => i.severity == ProblemSeverity.warning).length;
  int get infoCount =>
      items.where((i) => i.severity == ProblemSeverity.info).length;

  ProblemsState copyWith({List<ProblemItem>? items}) {
    return ProblemsState(items: items ?? this.items);
  }

  Map<String, dynamic> toJson() => {
        'items': items.map((i) => i.toJson()).toList(),
      };

  factory ProblemsState.fromJson(Map<String, dynamic> json) {
    final raw = json['items'] as List<dynamic>?;
    return ProblemsState(
      items: raw
              ?.map((e) => ProblemItem.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );
  }
}

enum ProblemSeverity { error, warning, info, hint }

class ProblemItem {
  const ProblemItem({
    required this.id,
    required this.message,
    required this.severity,
    this.path,
    this.line,
    this.column,
    this.source,
  });

  final String id;
  final String message;
  final ProblemSeverity severity;
  final String? path;
  final int? line;
  final int? column;
  final String? source;

  Map<String, dynamic> toJson() => {
        'id': id,
        'message': message,
        'severity': severity.name,
        'path': path,
        'line': line,
        'column': column,
        'source': source,
      };

  factory ProblemItem.fromJson(Map<String, dynamic> json) {
    return ProblemItem(
      id: json['id'] as String? ?? '',
      message: json['message'] as String? ?? '',
      severity: ProblemSeverity.values.firstWhere(
        (s) => s.name == json['severity'],
        orElse: () => ProblemSeverity.info,
      ),
      path: json['path'] as String?,
      line: json['line'] as int?,
      column: json['column'] as int?,
      source: json['source'] as String?,
    );
  }
}
