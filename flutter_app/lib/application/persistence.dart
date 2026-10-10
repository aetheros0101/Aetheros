import 'dart:async';
import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import '../state/app_state.dart';
import '../state/editor_state.dart';
import '../state/panel_state.dart';
import '../state/workbench_state.dart';
import '../navigation/navigation_state.dart';

/// Persists workbench layout + open editor tabs (not live domain service state).
///
/// Domain slices (workspace tree, git, agent sessions) are owned by Rust /
/// services and rehydrated on demand. UI layout is local.
class WorkbenchPersistence {
  WorkbenchPersistence({
    this.prefsKey = 'aetheros.workbench.v1',
    this.debounce = const Duration(milliseconds: 400),
  });

  final String prefsKey;
  final Duration debounce;

  Timer? _timer;
  SharedPreferences? _prefs;

  Future<SharedPreferences> _ensure() async {
    return _prefs ??= await SharedPreferences.getInstance();
  }

  /// Layout-only snapshot suitable for disk.
  static Map<String, dynamic> layoutSnapshot(AppState state) => {
        'workbench': state.workbench.toJson(),
        'navigation': {
          'activeId': state.navigation.activeId,
        },
        'editor': {
          'groups': state.editor.groups
              .map((g) => {
                    'activeTabIndex': g.activeTabIndex,
                    'tabs': g.tabs
                        .map((t) => {
                              'path': t.path,
                              'isPreview': t.isPreview,
                              'isPinned': t.isPinned,
                            })
                        .toList(),
                  })
              .toList(),
          'activeGroupIndex': state.editor.activeGroupIndex,
        },
        'panel': {
          'activePanelId': state.panel.activePanelId,
        },
      };

  Future<void> saveNow(AppState state) async {
    final prefs = await _ensure();
    final json = jsonEncode(layoutSnapshot(state));
    await prefs.setString(prefsKey, json);
  }

  /// Debounced save — call on every layout-affecting state change.
  void scheduleSave(AppState state) {
    _timer?.cancel();
    _timer = Timer(debounce, () {
      unawaited(saveNow(state));
    });
  }

  Future<AppState?> load() async {
    final prefs = await _ensure();
    final raw = prefs.getString(prefsKey);
    if (raw == null || raw.isEmpty) return null;
    try {
      final map = jsonDecode(raw) as Map<String, dynamic>;
      return _fromLayout(map);
    } catch (_) {
      return null;
    }
  }

  AppState _fromLayout(Map<String, dynamic> map) {
    WorkbenchLayoutState workbench = const WorkbenchLayoutState();
    if (map['workbench'] is Map<String, dynamic>) {
      workbench =
          WorkbenchLayoutState.fromJson(map['workbench'] as Map<String, dynamic>);
    }

    String? activeId;
    if (map['navigation'] is Map<String, dynamic>) {
      activeId = (map['navigation'] as Map)['activeId'] as String?;
    }

    EditorState editor = const EditorState();
    if (map['editor'] is Map<String, dynamic>) {
      final e = map['editor'] as Map<String, dynamic>;
      final groupsRaw = e['groups'] as List<dynamic>? ?? const [];
      final groups = <EditorGroupState>[];
      for (final g in groupsRaw) {
        if (g is! Map<String, dynamic>) continue;
        final tabsRaw = g['tabs'] as List<dynamic>? ?? const [];
        final tabs = <EditorTabState>[];
        for (final t in tabsRaw) {
          if (t is! Map<String, dynamic>) continue;
          final path = t['path'] as String? ?? '';
          if (path.isEmpty) continue;
          tabs.add(EditorTabState(
            id: path,
            path: path,
            isPreview: t['isPreview'] as bool? ?? false,
            isPinned: t['isPinned'] as bool? ?? false,
          ));
        }
        groups.add(EditorGroupState(
          tabs: tabs,
          activeTabIndex: g['activeTabIndex'] as int? ?? 0,
        ));
      }
      if (groups.isEmpty) groups.add(const EditorGroupState());
      editor = EditorState(
        groups: groups,
        activeGroupIndex: e['activeGroupIndex'] as int? ?? 0,
      );
    }

    PanelState panel = const PanelState();
    if (map['panel'] is Map<String, dynamic>) {
      final p = map['panel'] as Map<String, dynamic>;
      final id = p['activePanelId'] as String?;
      if (id != null) {
        panel = panel.activate(id);
      }
    }

    return AppState(
      workbench: workbench,
      navigation: activeId == null
          ? const NavigationState()
          : NavigationState(activeId: activeId),
      editor: editor,
      panel: panel,
    );
  }

  void dispose() {
    _timer?.cancel();
  }
}

/// Full AppState JSON encode/decode (tests / export).
abstract final class AppStatePersistence {
  static String encode(AppState state) => jsonEncode(state.toJson());

  static AppState decode(String raw) {
    try {
      final map = jsonDecode(raw) as Map<String, dynamic>;
      return AppState.fromJson(map);
    } catch (_) {
      return const AppState();
    }
  }
}
