/// AetherOS Application State — Git status summary for status bar & SCM view.
class GitState {
  const GitState({
    this.branch,
    this.isDirty = false,
    this.ahead = 0,
    this.behind = 0,
    this.changes = const [],
    this.isLoading = false,
    this.error,
  });

  final String? branch;
  final bool isDirty;
  final int ahead;
  final int behind;
  final List<GitChangeEntry> changes;
  final bool isLoading;
  final String? error;

  int get addedCount =>
      changes.where((c) => c.status == 'added' || c.status == 'untracked').length;
  int get modifiedCount =>
      changes.where((c) => c.status == 'modified').length;
  int get deletedCount =>
      changes.where((c) => c.status == 'deleted').length;

  GitState copyWith({
    String? branch,
    bool? isDirty,
    int? ahead,
    int? behind,
    List<GitChangeEntry>? changes,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return GitState(
      branch: branch ?? this.branch,
      isDirty: isDirty ?? this.isDirty,
      ahead: ahead ?? this.ahead,
      behind: behind ?? this.behind,
      changes: changes ?? this.changes,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }

  Map<String, dynamic> toJson() => {
        'branch': branch,
        'isDirty': isDirty,
        'ahead': ahead,
        'behind': behind,
      };

  factory GitState.fromJson(Map<String, dynamic> json) {
    return GitState(
      branch: json['branch'] as String?,
      isDirty: json['isDirty'] as bool? ?? false,
      ahead: json['ahead'] as int? ?? 0,
      behind: json['behind'] as int? ?? 0,
    );
  }
}

class GitChangeEntry {
  const GitChangeEntry({
    required this.path,
    required this.status,
    this.staged = false,
  });

  final String path;

  /// added | modified | deleted | renamed | untracked | conflict
  final String status;
  final bool staged;

  Map<String, dynamic> toJson() => {
        'path': path,
        'status': status,
        'staged': staged,
      };

  factory GitChangeEntry.fromJson(Map<String, dynamic> json) {
    return GitChangeEntry(
      path: json['path'] as String? ?? '',
      status: json['status'] as String? ?? 'modified',
      staged: json['staged'] as bool? ?? false,
    );
  }
}
