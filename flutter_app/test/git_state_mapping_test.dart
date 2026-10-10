import 'package:flutter_test/flutter_test.dart';
import 'package:aetheros_app/state/git_state.dart';

void main() {
  test('GitState counts by status', () {
    const g = GitState(changes: [
      GitChangeEntry(path: 'a.rs', status: 'added'),
      GitChangeEntry(path: 'b.rs', status: 'modified'),
      GitChangeEntry(path: 'c.rs', status: 'modified'),
      GitChangeEntry(path: 'd.rs', status: 'deleted'),
      GitChangeEntry(path: 'e.rs', status: 'untracked'),
    ]);
    expect(g.addedCount, 2); // added + untracked
    expect(g.modifiedCount, 2);
    expect(g.deletedCount, 1);
  });
}
