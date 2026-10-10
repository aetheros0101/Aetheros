import 'dart:async';

import '../../state/git_state.dart';

abstract class GitService {
  GitState get state;
  Stream<GitState> get changes;
  Future<void> refresh();
  Future<void> stage(String path);
  Future<void> unstage(String path);
  Future<void> commit(String message);
  void dispose();
}

class StubGitService implements GitService {
  StubGitService() {
    _state = const GitState(
      branch: 'main',
      isDirty: true,
      ahead: 1,
      behind: 0,
      changes: [
        GitChangeEntry(path: 'lib/app.dart', status: 'modified'),
        GitChangeEntry(path: 'README.md', status: 'added'),
      ],
    );
  }

  late GitState _state;
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
    _emit(_state);
  }

  @override
  Future<void> stage(String path) async {
    final updated = _state.changes
        .map((c) => c.path == path ? GitChangeEntry(path: c.path, status: c.status, staged: true) : c)
        .toList();
    _emit(_state.copyWith(changes: updated));
  }

  @override
  Future<void> unstage(String path) async {
    final updated = _state.changes
        .map((c) => c.path == path ? GitChangeEntry(path: c.path, status: c.status, staged: false) : c)
        .toList();
    _emit(_state.copyWith(changes: updated));
  }

  @override
  Future<void> commit(String message) async {
    _emit(_state.copyWith(changes: [], isDirty: false));
  }

  @override
  void dispose() {
    _controller.close();
  }
}
