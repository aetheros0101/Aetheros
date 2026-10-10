/// AetherOS Application State — Workspace / file tree model.
///
/// Flutter UI reads this model; population comes from WorkspaceService → Rust.
class WorkspaceState {
  const WorkspaceState({
    this.rootPath,
    this.rootName,
    this.entries = const [],
    this.expandedPaths = const {},
    this.selectedPath,
    this.isLoading = false,
    this.error,
  });

  final String? rootPath;
  final String? rootName;
  final List<WorkspaceEntry> entries;
  final Set<String> expandedPaths;
  final String? selectedPath;
  final bool isLoading;
  final String? error;

  bool isExpanded(String path) => expandedPaths.contains(path);

  WorkspaceState copyWith({
    String? rootPath,
    String? rootName,
    List<WorkspaceEntry>? entries,
    Set<String>? expandedPaths,
    String? selectedPath,
    bool? isLoading,
    String? error,
    bool clearError = false,
    bool clearSelection = false,
  }) {
    return WorkspaceState(
      rootPath: rootPath ?? this.rootPath,
      rootName: rootName ?? this.rootName,
      entries: entries ?? this.entries,
      expandedPaths: expandedPaths ?? this.expandedPaths,
      selectedPath:
          clearSelection ? null : (selectedPath ?? this.selectedPath),
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }

  WorkspaceState toggleExpanded(String path) {
    final next = Set<String>.from(expandedPaths);
    if (next.contains(path)) {
      next.remove(path);
    } else {
      next.add(path);
    }
    return copyWith(expandedPaths: next);
  }

  Map<String, dynamic> toJson() => {
        'rootPath': rootPath,
        'rootName': rootName,
        'expandedPaths': expandedPaths.toList(),
        'selectedPath': selectedPath,
      };

  factory WorkspaceState.fromJson(Map<String, dynamic> json) {
    final raw = json['expandedPaths'] as List<dynamic>?;
    return WorkspaceState(
      rootPath: json['rootPath'] as String?,
      rootName: json['rootName'] as String?,
      expandedPaths: raw?.map((e) => e as String).toSet() ?? {},
      selectedPath: json['selectedPath'] as String?,
    );
  }
}

enum WorkspaceEntryKind { folder, file }

class WorkspaceEntry {
  const WorkspaceEntry({
    required this.path,
    required this.name,
    required this.kind,
    this.children = const [],
    this.gitStatus,
  });

  final String path;
  final String name;
  final WorkspaceEntryKind kind;
  final List<WorkspaceEntry> children;

  /// Git decoration letter: A, M, D, R, U, C, or null.
  final String? gitStatus;

  bool get isFolder => kind == WorkspaceEntryKind.folder;

  WorkspaceEntry copyWith({
    String? path,
    String? name,
    WorkspaceEntryKind? kind,
    List<WorkspaceEntry>? children,
    String? gitStatus,
  }) {
    return WorkspaceEntry(
      path: path ?? this.path,
      name: name ?? this.name,
      kind: kind ?? this.kind,
      children: children ?? this.children,
      gitStatus: gitStatus ?? this.gitStatus,
    );
  }
}
