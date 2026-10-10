/// Agent verification pipeline result (post-task checks).
enum VerificationOutcome { pending, passed, failed, skipped }

class VerificationReport {
  const VerificationReport({
    this.items = const [],
    this.completedAt,
  });

  final List<VerificationCheck> items;
  final String? completedAt;

  bool get allPassed =>
      items.isNotEmpty && items.every((i) => i.outcome == VerificationOutcome.passed);

  Map<String, dynamic> toJson() => {
        'items': items.map((i) => i.toJson()).toList(),
        'completedAt': completedAt,
      };
}

class VerificationCheck {
  const VerificationCheck({
    required this.id,
    required this.title,
    this.outcome = VerificationOutcome.pending,
    this.detail,
  });

  final String id;
  final String title;
  final VerificationOutcome outcome;
  final String? detail;

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'outcome': outcome.name,
        'detail': detail,
      };
}
