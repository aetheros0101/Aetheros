import 'dart:async';

import '../../api/aetheros_api.dart';
import '../../state/git_state.dart';
import 'git_service.dart';

/// Git via workspace_core only: AetherApi.gitStatus / stage / unstage / commit / diff.
class FrbGitService implements GitService {
  GitState _state = const GitState();
  final _controller = StreamController<GitState>.broadcast();

  @override
  GitState get state => _state;

  @override
  Stream<GitState> get changes => _controller.stream;

  void _emit(GitState next) {
    _state = next;
    if (!_controller.isClosed) _controller.add(next);
  }

  @override
  Future<void> refresh() async {
    _emit(_state.copyWith(isLoading: true, clearError: true));
    try {
      final s = await AetherApi.gitStatus();
      _emit(GitState(
        branch: s.branch,
        isDirty: s.entries.isNotEmpty,
        ahead: s.ahead,
        behind: s.behind,
        changes: [
          for (final e in s.entries)
            GitChangeEntry(
              path: e.path,
              status: e.kind,
              staged: e.staged,
            ),
        ],
        isLoading: false,
      ));
    } catch (e) {
      _emit(_state.copyWith(isLoading: false, error: e.toString()));
    }
  }

  Map<String, String> decorationMap() {
    final map = <String, String>{};
    for (final c in _state.changes) {
      map[c.path] = switch (c.status) {
        'added' => 'A',
        'modified' => 'M',
        'deleted' => 'D',
        'renamed' => 'R',
        'untracked' => 'U',
        'conflict' => 'C',
        _ => 'M',
      };
    }
    return map;
  }

  Future<dynamic> diff({bool staged = false, String? path}) =>
      AetherApi.gitDiff(staged: staged, path: path);

  @override
  Future<void> stage(String path) async {
    try {
      await AetherApi.gitStage(path);
      await refresh();
    } catch (e) {
      _emit(_state.copyWith(error: e.toString(), isLoading: false));
    }
  }

  @override
  Future<void> unstage(String path) async {
    try {
      await AetherApi.gitUnstage(path);
      await refresh();
    } catch (e) {
      _emit(_state.copyWith(error: e.toString(), isLoading: false));
    }
  }

  @override
  Future<void> commit(String message) async {
    try {
      await AetherApi.gitCommit(message);
      await refresh();
    } catch (e) {
      _emit(_state.copyWith(error: e.toString(), isLoading: false));
    }
  }

  @override
  void dispose() {
    _controller.close();
  }
}
