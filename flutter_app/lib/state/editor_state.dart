/// AetherOS Application State — Editor Groups & Tabs
enum EditorSplitDirection { horizontal, vertical }

class EditorState {
  const EditorState({
    this.groups = const [EditorGroupState()],
    this.activeGroupIndex = 0,
    this.splitDirection = EditorSplitDirection.horizontal,
  });

  final List<EditorGroupState> groups;
  final int activeGroupIndex;
  final EditorSplitDirection splitDirection;

  EditorGroupState get activeGroup => groups.isEmpty
      ? const EditorGroupState()
      : groups[activeGroupIndex.clamp(0, groups.length - 1)];

  EditorState copyWith({
    List<EditorGroupState>? groups,
    int? activeGroupIndex,
    EditorSplitDirection? splitDirection,
  }) {
    return EditorState(
      groups: groups ?? this.groups,
      activeGroupIndex: activeGroupIndex ?? this.activeGroupIndex,
      splitDirection: splitDirection ?? this.splitDirection,
    );
  }

  EditorState openFile(String path, {bool preview = false}) {
    final group = activeGroup;
    final existing = group.tabs.indexWhere((t) => t.path == path);
    if (existing >= 0) {
      return _updateActiveGroup(group.copyWith(activeTabIndex: existing));
    }
    final tab = EditorTabState(id: path, path: path, isPreview: preview);
    final tabs = preview
        ? [...group.tabs.where((t) => !t.isPreview), tab]
        : [...group.tabs, tab];
    return _updateActiveGroup(
      group.copyWith(tabs: tabs, activeTabIndex: tabs.length - 1),
    );
  }

  /// Close tab at [tabIndex] in [groupIndex].
  /// Active tab selection is deterministic:
  /// - closing a tab before the active → active index shifts left by 1
  /// - closing the active tab → same index (next file) or previous if last
  /// - closing a tab after the active → active index unchanged
  EditorState closeTab(int groupIndex, int tabIndex) {
    if (groupIndex < 0 || groupIndex >= groups.length) return this;
    final group = groups[groupIndex];
    if (tabIndex < 0 || tabIndex >= group.tabs.length) return this;
    final tabs = [...group.tabs]..removeAt(tabIndex);

    int newActive;
    if (tabs.isEmpty) {
      newActive = 0;
    } else if (tabIndex < group.activeTabIndex) {
      newActive = group.activeTabIndex - 1;
    } else if (tabIndex == group.activeTabIndex) {
      // Prefer the tab that slides into this index; else previous.
      newActive = tabIndex.clamp(0, tabs.length - 1);
    } else {
      newActive = group.activeTabIndex;
    }
    newActive = newActive.clamp(0, tabs.isEmpty ? 0 : tabs.length - 1);

    final newGroups = [...groups];
    newGroups[groupIndex] = group.copyWith(tabs: tabs, activeTabIndex: newActive);
    final filtered = newGroups.where((g) => g.tabs.isNotEmpty).toList();
    final result = filtered.isEmpty ? [const EditorGroupState()] : filtered;
    return copyWith(
      groups: result,
      activeGroupIndex: activeGroupIndex.clamp(0, result.length - 1),
    );
  }

  /// Whether [path] is still open in any group.
  bool isPathOpen(String path) {
    for (final g in groups) {
      for (final t in g.tabs) {
        if (t.path == path) return true;
      }
    }
    return false;
  }

  EditorState closeActiveTab() =>
      closeTab(activeGroupIndex, activeGroup.activeTabIndex);

  EditorState nextTab() {
    final group = activeGroup;
    if (group.tabs.length < 2) return this;
    final next = (group.activeTabIndex + 1) % group.tabs.length;
    return _updateActiveGroup(group.copyWith(activeTabIndex: next));
  }

  EditorState previousTab() {
    final group = activeGroup;
    if (group.tabs.length < 2) return this;
    final prev = (group.activeTabIndex - 1 + group.tabs.length) % group.tabs.length;
    return _updateActiveGroup(group.copyWith(activeTabIndex: prev));
  }

  EditorState split() {
    final active = activeGroup;
    final tab = active.activeTab;
    if (tab == null) return this;
    final newGroup = EditorGroupState(tabs: [tab], activeTabIndex: 0);
    return copyWith(groups: [...groups, newGroup], activeGroupIndex: groups.length);
  }

  EditorState _updateActiveGroup(EditorGroupState group) {
    final newGroups = [...groups];
    newGroups[activeGroupIndex] = group;
    return copyWith(groups: newGroups);
  }

  Map<String, dynamic> toJson() => {
        'groups': groups.map((g) => g.toJson()).toList(),
        'activeGroupIndex': activeGroupIndex,
        'splitDirection': splitDirection.name,
      };

  factory EditorState.fromJson(Map<String, dynamic> json) {
    final rawGroups = json['groups'] as List<dynamic>?;
    return EditorState(
      groups: rawGroups == null || rawGroups.isEmpty
          ? const [EditorGroupState()]
          : rawGroups
              .map((e) => EditorGroupState.fromJson(e as Map<String, dynamic>))
              .toList(),
      activeGroupIndex: json['activeGroupIndex'] as int? ?? 0,
      splitDirection: EditorSplitDirection.values.firstWhere(
        (d) => d.name == json['splitDirection'],
        orElse: () => EditorSplitDirection.horizontal,
      ),
    );
  }
}

class EditorGroupState {
  const EditorGroupState({this.tabs = const [], this.activeTabIndex = 0});
  final List<EditorTabState> tabs;
  final int activeTabIndex;
  EditorTabState? get activeTab =>
      tabs.isEmpty ? null : tabs[activeTabIndex.clamp(0, tabs.length - 1)];
  EditorGroupState copyWith({List<EditorTabState>? tabs, int? activeTabIndex}) =>
      EditorGroupState(tabs: tabs ?? this.tabs, activeTabIndex: activeTabIndex ?? this.activeTabIndex);
  Map<String, dynamic> toJson() => {'tabs': tabs.map((t) => t.toJson()).toList(), 'activeTabIndex': activeTabIndex};
  factory EditorGroupState.fromJson(Map<String, dynamic> json) {
    final rawTabs = json['tabs'] as List<dynamic>?;
    return EditorGroupState(
      tabs: rawTabs == null ? const [] : rawTabs.map((e) => EditorTabState.fromJson(e as Map<String, dynamic>)).toList(),
      activeTabIndex: json['activeTabIndex'] as int? ?? 0,
    );
  }
}

class EditorTabState {
  const EditorTabState({required this.id, required this.path, this.isDirty = false, this.isPreview = false, this.isPinned = false});
  final String id;
  final String path;
  final bool isDirty;
  final bool isPreview;
  final bool isPinned;
  String get fileName {
    final parts = path.replaceAll(r'\\', '/').split('/');
    return parts.isEmpty ? path : parts.last;
  }
  EditorTabState copyWith({String? id, String? path, bool? isDirty, bool? isPreview, bool? isPinned}) =>
      EditorTabState(id: id ?? this.id, path: path ?? this.path, isDirty: isDirty ?? this.isDirty, isPreview: isPreview ?? this.isPreview, isPinned: isPinned ?? this.isPinned);
  Map<String, dynamic> toJson() => {'id': id, 'path': path, 'isDirty': isDirty, 'isPreview': isPreview, 'isPinned': isPinned};
  factory EditorTabState.fromJson(Map<String, dynamic> json) => EditorTabState(
        id: json['id'] as String? ?? '', path: json['path'] as String? ?? '',
        isDirty: json['isDirty'] as bool? ?? false, isPreview: json['isPreview'] as bool? ?? false,
        isPinned: json['isPinned'] as bool? ?? false);
}
